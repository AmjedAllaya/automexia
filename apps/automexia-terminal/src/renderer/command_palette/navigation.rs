//! Capability-free grouping of the existing application command catalog.

use super::{
    CommandIcon, PaletteAction, RowPresentation, BRAND_AMBER, BRAND_BLUE, BRAND_CYAN,
    BRAND_LIME, BRAND_PURPLE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Category {
    Tabs,
    Panes,
    Search,
    Input,
    Appearance,
    Tools,
}

impl Category {
    pub(super) const ALL: [Self; 6] = [
        Self::Tabs,
        Self::Panes,
        Self::Search,
        Self::Input,
        Self::Appearance,
        Self::Tools,
    ];

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Tabs => "Tabs & Windows",
            Self::Panes => "Panes & Sessions",
            Self::Search => "Search & History",
            Self::Input => "Clipboard & Input",
            Self::Appearance => "Appearance",
            Self::Tools => "Tools",
        }
    }

    pub(super) fn presentation(self) -> RowPresentation {
        let (icon, accent) = match self {
            Self::Tabs => (CommandIcon::TabAdd, BRAND_CYAN),
            Self::Panes => (CommandIcon::SplitRight, BRAND_PURPLE),
            Self::Search => (CommandIcon::Search, BRAND_BLUE),
            Self::Input => (CommandIcon::Paste, BRAND_LIME),
            Self::Appearance => (CommandIcon::Theme, BRAND_AMBER),
            Self::Tools => (CommandIcon::Settings, BRAND_CYAN),
        };
        RowPresentation { icon, accent }
    }

    // Exhaustive mapping: a new action cannot silently disappear from browsing.
    pub(super) fn for_action(action: PaletteAction) -> Self {
        use PaletteAction::*;
        match action {
            TabCreate | LocalTabCreate | TabClose | TabCloseUnfocused | SelectNextTab
            | SelectPrevTab | SelectNextLocalTab | SelectPrevLocalTab
            | WindowCreateNew | Quit => Self::Tabs,
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
            ScrollToPreviousCommand
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
            | ListFonts => Self::Appearance,
            ConfigEditor | PreviewSelectedImage | ViewTableOutput | OpenMarket
            | OpenConnections | OpenActions => Self::Tools,
        }
    }
}
