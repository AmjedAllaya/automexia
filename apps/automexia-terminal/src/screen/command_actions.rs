//! Explicit UI effects read the live grid only when a choice is activated.
use super::*;
use crate::context::command_actions::CommandTarget;
use crate::renderer::command_palette::LastCommandAction;

impl Screen<'_> {
    pub(crate) fn open_last_command_actions(&mut self) {
        self.close_settings_view();
        // Closing the palette after insertion must return input to the shell,
        // never to a search field or a terminal navigation mode.
        self.search_state.dfas = None;
        self.exit_search();
        self.stop_hint_mode_if_active();
        self.dismiss_suggestions(
            crate::automexia::suggestions::SuggestionInvalidation::ModalOpened,
        );
        self.dismiss_image_preview();
        self.table_view.close();
        let target = self.context_manager.current().command_target();
        self.renderer
            .command_palette
            .enter_last_command_actions(target);
        self.mark_dirty();
    }
    pub(crate) fn apply_last_command_action(
        &mut self,
        target: CommandTarget,
        action: LastCommandAction,
        clipboard: &mut Clipboard,
    ) {
        let outcome = self.perform_last_command_action(target, action, clipboard);
        match outcome {
            Ok(()) => self.renderer.command_palette.set_enabled(false),
            Err(message) => self.renderer.command_palette.last_command_notice(message),
        }
        self.mark_dirty();
    }
    fn perform_last_command_action(
        &mut self,
        target: CommandTarget,
        action: LastCommandAction,
        clipboard: &mut Clipboard,
    ) -> Result<(), &'static str> {
        use rio_backend::crosswords::command_actions::CommandActionError;
        if action == LastCommandAction::Reinsert {
            return self
                .context_manager
                .reinsert_command(target)
                .map_err(CommandActionError::message);
        }
        let context = self.context_manager.current_mut();
        if !context.matches_command_target(target) {
            return Err(CommandActionError::Stale.message());
        }
        let mut terminal = context.terminal.lock();
        match action {
            LastCommandAction::CopyCommand | LastCommandAction::CopyOutput => {
                let text = if action == LastCommandAction::CopyCommand {
                    terminal.command_text(target.handle)
                } else {
                    terminal.command_output(target.handle)
                }
                .map_err(CommandActionError::message)?;
                drop(terminal);
                if text.is_empty() {
                    return Err("No text in this command range. Clipboard unchanged.");
                }
                clipboard
                    .try_set(ClipboardType::Clipboard, text)
                    .map_err(|_| "Clipboard unavailable. No text was copied.")?;
            }
            LastCommandAction::SelectOutput => {
                // Apply the same completeness, privacy and size checks as Copy.
                terminal
                    .command_output(target.handle)
                    .map_err(CommandActionError::message)?;
                let ranges = terminal
                    .command_ranges(target.handle)
                    .map_err(CommandActionError::message)?;
                let Some((start, end)) = ranges.output else {
                    return Err("This command has no output to select.");
                };
                let mut selection =
                    Selection::new(SelectionType::Simple, start, Side::Left);
                selection.update(end, Side::Right);
                let range = selection.to_range(&terminal);
                terminal.selection = Some(selection);
                let offset = terminal.grid.display_offset() as i32;
                terminal.scroll_display(Scroll::Delta((-start.row.0).max(0) - offset));
                drop(terminal);
                context.set_selection(range);
                context.renderable_content.command_rows.follow();
            }
            LastCommandAction::Jump => {
                let ranges = terminal
                    .command_ranges(target.handle)
                    .map_err(CommandActionError::message)?;
                let offset = terminal.grid.display_offset() as i32;
                terminal.scroll_display(Scroll::Delta(
                    (-ranges.prompt.row.0).max(0) - offset,
                ));
                drop(terminal);
                context.renderable_content.command_rows.follow();
            }
            LastCommandAction::Reinsert => unreachable!("handled before grid read"),
        }
        self.context_manager.request_render();
        Ok(())
    }
}
