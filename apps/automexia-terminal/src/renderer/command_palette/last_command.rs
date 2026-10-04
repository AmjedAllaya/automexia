//! Compact choices only; command/output contents never enter the palette model.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LastCommandAction {
    CopyCommand,
    CopyOutput,
    SelectOutput,
    Reinsert,
    Jump,
}
impl LastCommandAction {
    pub(super) const ALL: [Self; 5] = [
        Self::CopyCommand,
        Self::CopyOutput,
        Self::SelectOutput,
        Self::Reinsert,
        Self::Jump,
    ];
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::CopyCommand => "Copy command",
            Self::CopyOutput => "Copy output",
            Self::SelectOutput => "Select output",
            Self::Reinsert => "Reinsert command",
            Self::Jump => "Jump to start",
        }
    }
    pub(super) fn presentation(self) -> RowPresentation {
        RowPresentation {
            icon: match self {
                Self::CopyCommand | Self::CopyOutput => CommandIcon::Copy,
                Self::SelectOutput => CommandIcon::Search,
                Self::Reinsert => CommandIcon::Paste,
                Self::Jump => CommandIcon::History,
            },
            accent: UiAccent::Cyan,
        }
    }
}
impl CommandPalette {
    pub(crate) fn enter_last_command_actions(&mut self, target: Option<CommandTarget>) {
        self.set_enabled(true);
        self.mode = PaletteMode::LastCommandActions {
            target,
            notice: if target.is_some() {
                "Reinsert adds text only. Press Enter in the shell to run it."
            } else {
                "No completed shell-integrated command is available."
            },
        };
        self.set_query(String::new());
        self.selected_index = usize::from(target.is_some());
    }
    pub(crate) fn last_command_notice(&mut self, message: &'static str) {
        if let PaletteMode::LastCommandActions { notice, .. } = &mut self.mode {
            *notice = message;
            self.query.clear();
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
    }
    pub(crate) fn selected_last_command_action(
        &self,
    ) -> Option<(CommandTarget, LastCommandAction)> {
        let PaletteMode::LastCommandActions {
            target: Some(target),
            ..
        } = self.mode
        else {
            return None;
        };
        self.filtered_rows()
            .get(self.selected_index)
            .and_then(|(_, row)| match row {
                PaletteRow::LastCommand(action) => Some((target, *action)),
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target() -> CommandTarget {
        use rio_backend::{
            ansi::CursorShape,
            crosswords::{Crosswords, CrosswordsSize},
            event::{VoidListener, WindowId},
            performer::handler::Processor,
        };
        let mut t = Crosswords::new(
            CrosswordsSize::new(80, 12),
            CursorShape::Block,
            VoidListener {},
            WindowId::from(0),
            0,
            128,
        );
        Processor::default().advance(&mut t, b"\x1b]133;A;aid=1\x07> \x1b]133;B\x07echo sample\r\n\x1b]133;C\x07sample\r\n\x1b]133;D;0\x07\x1b]133;A;aid=2\x07> \x1b]133;B\x07");
        CommandTarget {
            route_id: 7,
            rich_text_id: 9,
            handle: t.last_command_handle().unwrap(),
        }
    }
    #[test]
    fn command_actions_palette_lists_exact_actions_without_terminal_contents() {
        let mut palette = CommandPalette::new();
        let target = target();
        palette.enter_last_command_actions(Some(target));
        assert_eq!(
            palette.selected_last_command_action(),
            Some((target, LastCommandAction::CopyCommand))
        );
        assert_eq!(
            palette
                .filtered_rows()
                .iter()
                .skip(1)
                .map(|(_, row)| row.title())
                .collect::<Vec<_>>(),
            vec![
                "Copy command",
                "Copy output",
                "Select output",
                "Reinsert command",
                "Jump to start"
            ]
        );
        palette.move_selection_down();
        assert_eq!(
            palette.selected_last_command_action(),
            Some((target, LastCommandAction::CopyOutput))
        );
        palette.set_query("reinsert".into());
        assert_eq!(
            palette.selected_last_command_action(),
            Some((target, LastCommandAction::Reinsert))
        );
        assert!(palette.go_back());
        assert_eq!(
            palette.get_selected_action(),
            Some(PaletteAction::LastCommandActions)
        );
    }
    #[test]
    fn command_actions_palette_no_integration_and_error_notices_are_not_actions() {
        let mut palette = CommandPalette::new();
        palette.enter_last_command_actions(None);
        assert_eq!(palette.filtered_rows().len(), 1);
        assert!(palette.get_selected_action().is_none());
        assert!(palette.selected_last_command_action().is_none());
        palette.enter_last_command_actions(Some(target()));
        palette.set_query("copy".into());
        palette.last_command_notice("The terminal changed. Reopen Last-command actions.");
        assert!(palette.selected_last_command_action().is_none());
        assert_eq!(
            palette.filtered_rows()[0].1.title(),
            "The terminal changed. Reopen Last-command actions."
        );
        assert!(palette.can_go_back());
    }
}
