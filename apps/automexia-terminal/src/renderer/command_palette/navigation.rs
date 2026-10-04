//! Capability-free grouping of the existing application command catalog.

use super::{CommandIcon, PaletteAction, RowPresentation, UiAccent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Category {
    Tabs,
    Panes,
    Search,
    Input,
    Appearance,
    Customizations,
    Tools,
}

impl Category {
    pub(super) const ALL: [Self; 7] = [
        Self::Tabs,
        Self::Panes,
        Self::Search,
        Self::Input,
        Self::Appearance,
        Self::Customizations,
        Self::Tools,
    ];

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Tabs => "Tabs & Windows",
            Self::Panes => "Panes & Sessions",
            Self::Search => "Search & History",
            Self::Input => "Clipboard & Input",
            Self::Appearance => "Appearance",
            Self::Customizations => "Customizations",
            Self::Tools => "Tools",
        }
    }

    pub(super) fn presentation(self) -> RowPresentation {
        let (icon, accent) = match self {
            Self::Tabs => (CommandIcon::TabAdd, UiAccent::Cyan),
            Self::Panes => (CommandIcon::SplitRight, UiAccent::Purple),
            Self::Search => (CommandIcon::Search, UiAccent::Blue),
            Self::Input => (CommandIcon::Paste, UiAccent::Success),
            Self::Appearance => (CommandIcon::Theme, UiAccent::Warning),
            Self::Customizations => (CommandIcon::Customizations, UiAccent::Cyan),
            Self::Tools => (CommandIcon::Toolbox, UiAccent::Cyan),
        };
        RowPresentation { icon, accent }
    }

    // Exhaustive mapping: a new action cannot silently disappear from browsing.
    pub(super) fn for_action(action: PaletteAction) -> Self {
        use PaletteAction::*;
        match action {
            TabCreate
            | LocalTabCreate
            | TabClose
            | TabCloseUnfocused
            | SelectNextTab
            | SelectPrevTab
            | SelectNextLocalTab
            | SelectPrevLocalTab
            | WindowCreateNew
            | RestorePreviousSession
            | Quit => Self::Tabs,
            SplitRight
            | SplitDown
            | CloneSplitRight
            | CloneSplitDown
            | SelectNextSplit
            | SelectPrevSplit
            | SelectPaneLeft
            | SelectPaneRight
            | SelectPaneUp
            | SelectPaneDown
            | CloseCurrentSplitOrTab => Self::Panes,
            LastCommandActions
            | ScrollToPreviousCommand
            | ScrollToNextCommand
            | SearchForward
            | SearchBackward
            | SearchGlobalForward
            | SearchGlobalBackward
            | ClearScreen => Self::Search,
            Copy | Paste | ToggleViMode => Self::Input,
            IncreaseFontSize
            | DecreaseFontSize
            | ResetFontSize
            | ToggleFullscreen
            | ToggleAppearanceTheme
            | OpenThemeGallery
            | ListFonts => Self::Appearance,
            OpenSettings | OpenCustomizations => Self::Customizations,
            ConfigEditor | PreviewSelectedImage | ViewTableOutput | OpenMarket
            | OpenConnections | OpenActions => Self::Tools,
        }
    }
}
