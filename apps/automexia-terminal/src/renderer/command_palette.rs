// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::automexia::marketplace::MarketItem;
use crate::renderer::responsive::{elide_end, elide_start, Viewport};
use crate::renderer::scrollbar;
use crate::renderer::ui_theme::{
    color_u8, BORDER as OUTLINE_COLOR, BRAND_AMBER, BRAND_BLUE, BRAND_CORAL, BRAND_CYAN,
    BRAND_LIME, BRAND_PURPLE, CARD as BG_COLOR, CARD_RADIUS, CONTROL_RADIUS,
    KEYCAP_RADIUS, MODAL_SCRIM as BACKDROP_COLOR, MODAL_SHADOW as SHADOW_COLOR,
    MUTED_TEXT as DIM_TEXT_COLOR, MUTED_TEXT as SHORTCUT_TEXT_COLOR,
    OUTLINE as INPUT_OUTLINE_COLOR, SURFACE as INPUT_BG_COLOR,
    SURFACE as SHORTCUT_BG_COLOR, SURFACE_RAISED as SELECTED_BG_COLOR,
    TEXT as TEXT_COLOR,
};
use automexia_ui_model::quick_actions::{
    QuickActionListItem, QuickActionReviewView, QuickActionRisk,
};
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;
use std::time::Instant;

mod navigation;
#[cfg(test)]
mod navigation_tests;
use navigation::Category;

// Headerless command palette: search is the visual anchor and every action
// shares one crisp, DPI-independent icon grid.
const PALETTE_WIDTH: f32 = 600.0;
const PALETTE_CORNER_RADIUS: f32 = CARD_RADIUS;
const PALETTE_MARGIN_TOP: f32 = 76.0;
const PALETTE_PADDING: f32 = 12.0;

const INPUT_HEIGHT: f32 = 52.0;
const INPUT_FONT_SIZE: f32 = 15.0;
const INPUT_PADDING_X: f32 = 16.0;
const INPUT_ICON_WELL: f32 = 32.0;
const ESC_BADGE_WIDTH: f32 = 42.0;

const RESULT_ITEM_HEIGHT: f32 = 44.0;
const RESULT_FONT_SIZE: f32 = 14.5;
const RESULT_ICON_SIZE: f32 = 22.0;
const SHORTCUT_FONT_SIZE: f32 = 11.0;
const MAX_VISIBLE_RESULTS: usize = 10;
const MAX_PALETTE_QUERY_BYTES: usize = 4 * 1024;
const PALETTE_SCROLLBAR_IDLE_OPACITY: f32 = 0.55;
const MAX_WHEEL_ROWS_PER_EVENT: f64 = 1_024.0;

// Copy icon (two overlapping page outlines with rounded corners,
// drawn by layering filled + cutout rounded rects). Sized to fit
// comfortably inside RESULT_ITEM_HEIGHT.
const COPY_ICON_PAGE_W: f32 = 10.0;
const COPY_ICON_PAGE_H: f32 = 12.0;
const COPY_ICON_OFFSET: f32 = 3.0;
const COPY_ICON_STROKE: f32 = 1.0;
const COPY_ICON_RADIUS: f32 = 2.0;
const COPY_ICON_W: f32 = COPY_ICON_PAGE_W + COPY_ICON_OFFSET; // 13
const COPY_ICON_H: f32 = COPY_ICON_PAGE_H + COPY_ICON_OFFSET; // 15

const SEPARATOR_HEIGHT: f32 = 1.0;
const RESULTS_MARGIN_TOP: f32 = 8.0;
const CARET_WIDTH: f32 = 1.5;
const CARET_BLINK_MS: u128 = 500;

const SEPARATOR_COLOR: [f32; 4] = OUTLINE_COLOR;

fn trailing_label_max_width(input_width: f32) -> f32 {
    if !input_width.is_finite() {
        return 0.0;
    }
    (input_width * 0.42).clamp(0.0, 220.0)
}

#[inline]
fn bounded_scroll_offset(total: usize, visible: usize, requested: usize) -> usize {
    requested.min(total.saturating_sub(visible.max(1)))
}

// Depth / order
const DEPTH_BACKDROP: f32 = 0.0;
const DEPTH_BG: f32 = 0.1;
const DEPTH_ELEMENT: f32 = 0.2;
const ORDER: u8 = 20;

#[cfg(target_os = "macos")]
const SHORTCUT_NEW_TAB: &str = "Cmd+T";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_NEW_TAB: &str = "Ctrl+T";
#[cfg(target_os = "macos")]
const SHORTCUT_NEW_LOCAL_TAB: &str = "Cmd+Shift+T";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_NEW_LOCAL_TAB: &str = "Ctrl+Shift+T";
#[cfg(target_os = "macos")]
const SHORTCUT_CLOSE_TAB: &str = "Cmd+Shift+W";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CLOSE_TAB: &str = "Ctrl+F4";
#[cfg(target_os = "macos")]
const SHORTCUT_CLOSE_OTHER_TABS: &str = "Cmd+Alt+W";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CLOSE_OTHER_TABS: &str = "Ctrl+Shift+F4";
#[cfg(target_os = "macos")]
const SHORTCUT_CLOSE_SURFACE: &str = "Cmd+W";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CLOSE_SURFACE: &str = "Ctrl+Shift+W";
#[cfg(target_os = "macos")]
const SHORTCUT_SPLIT_RIGHT: &str = "Cmd+D";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SPLIT_RIGHT: &str = "Alt+Shift+R";
#[cfg(target_os = "macos")]
const SHORTCUT_SPLIT_DOWN: &str = "Cmd+Shift+D";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SPLIT_DOWN: &str = "Alt+Shift+D";
#[cfg(target_os = "macos")]
const SHORTCUT_CLONE_RIGHT: &str = "Cmd+Alt+Shift+R";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CLONE_RIGHT: &str = "Alt+R";
#[cfg(target_os = "macos")]
const SHORTCUT_CLONE_DOWN: &str = "Cmd+Alt+Shift+D";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CLONE_DOWN: &str = "Alt+D";
#[cfg(target_os = "macos")]
const SHORTCUT_PREV_LOCAL_TAB: &str = "Cmd+Alt+[";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PREV_LOCAL_TAB: &str = "Alt+PageUp";
#[cfg(target_os = "macos")]
const SHORTCUT_NEXT_LOCAL_TAB: &str = "Cmd+Alt+]";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_NEXT_LOCAL_TAB: &str = "Alt+PageDown";
#[cfg(target_os = "macos")]
const SHORTCUT_PANE_LEFT: &str = "Cmd+Alt+Left";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PANE_LEFT: &str = "Alt+Left";
#[cfg(target_os = "macos")]
const SHORTCUT_PANE_RIGHT: &str = "Cmd+Alt+Right";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PANE_RIGHT: &str = "Alt+Right";
#[cfg(target_os = "macos")]
const SHORTCUT_PANE_UP: &str = "Cmd+Alt+Up";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PANE_UP: &str = "Alt+Up";
#[cfg(target_os = "macos")]
const SHORTCUT_PANE_DOWN: &str = "Cmd+Alt+Down";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PANE_DOWN: &str = "Alt+Down";
#[cfg(target_os = "macos")]
const SHORTCUT_NEXT_PANE: &str = "Cmd+]";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_NEXT_PANE: &str = "F6";
#[cfg(target_os = "macos")]
const SHORTCUT_PREV_PANE: &str = "Cmd+[";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PREV_PANE: &str = "Shift+F6";
#[cfg(target_os = "macos")]
const SHORTCUT_SETTINGS: &str = "Cmd+,";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SETTINGS: &str = "Ctrl+,";
#[cfg(target_os = "macos")]
const SHORTCUT_NEW_WINDOW: &str = "Cmd+N";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_NEW_WINDOW: &str = "Ctrl+Shift+N";
#[cfg(target_os = "macos")]
const SHORTCUT_COPY: &str = "Cmd+C";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_COPY: &str = "Ctrl+Shift+C";
#[cfg(target_os = "macos")]
const SHORTCUT_PASTE: &str = "Cmd+V";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PASTE: &str = "Ctrl+Shift+V";
#[cfg(target_os = "macos")]
const SHORTCUT_PREVIOUS_COMMAND: &str = "Cmd+Shift+Up";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PREVIOUS_COMMAND: &str = "Ctrl+Shift+Up";
#[cfg(target_os = "macos")]
const SHORTCUT_NEXT_COMMAND: &str = "Cmd+Shift+Down";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_NEXT_COMMAND: &str = "Ctrl+Shift+Down";
#[cfg(target_os = "macos")]
const SHORTCUT_SEARCH: &str = "Cmd+F";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SEARCH: &str = "Ctrl+F";
#[cfg(target_os = "macos")]
const SHORTCUT_SEARCH_BACKWARD: &str = "Cmd+B";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SEARCH_BACKWARD: &str = "Alt+Shift+B";
#[cfg(target_os = "macos")]
const SHORTCUT_SEARCH_GLOBAL: &str = "Cmd+Shift+F";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SEARCH_GLOBAL: &str = "Ctrl+Shift+F";
#[cfg(target_os = "macos")]
const SHORTCUT_SEARCH_GLOBAL_BACKWARD: &str = "Cmd+Shift+B";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SEARCH_GLOBAL_BACKWARD: &str = "Ctrl+Shift+B";
#[cfg(target_os = "macos")]
const SHORTCUT_FONT_UP: &str = "Cmd++";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_FONT_UP: &str = "Ctrl++";
#[cfg(target_os = "macos")]
const SHORTCUT_FONT_DOWN: &str = "Cmd+-";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_FONT_DOWN: &str = "Ctrl+-";
#[cfg(target_os = "macos")]
const SHORTCUT_FONT_RESET: &str = "Cmd+0";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_FONT_RESET: &str = "Ctrl+0";
#[cfg(target_os = "macos")]
const SHORTCUT_VI_MODE: &str = "Alt+Shift+Space";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_VI_MODE: &str = "Ctrl+Shift+Space";
#[cfg(target_os = "macos")]
const SHORTCUT_FULLSCREEN: &str = "Ctrl+Cmd+F";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_FULLSCREEN: &str = "F11";
#[cfg(target_os = "macos")]
const SHORTCUT_APPEARANCE: &str = "Cmd+Alt+Shift+T";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_APPEARANCE: &str = "Alt+Shift+T";
#[cfg(target_os = "macos")]
const SHORTCUT_CONNECTION_HUB: &str = "Cmd+Shift+H";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CONNECTION_HUB: &str = "Ctrl+Shift+H";
#[cfg(target_os = "macos")]
const SHORTCUT_QUICK_ACTIONS: &str = "Cmd+Shift+O";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_QUICK_ACTIONS: &str = "Ctrl+Shift+O";
#[cfg(target_os = "macos")]
const SHORTCUT_EXTENSIONS: &str = "Cmd+Shift+M";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_EXTENSIONS: &str = "Ctrl+Shift+M";
#[cfg(target_os = "macos")]
const SHORTCUT_FONT_BROWSER: &str = "Cmd+Shift+L";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_FONT_BROWSER: &str = "Ctrl+Shift+L";
#[cfg(target_os = "macos")]
const SHORTCUT_PREVIEW_IMAGE: &str = "Cmd+Alt+I";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PREVIEW_IMAGE: &str = "Ctrl+Alt+I";
#[cfg(target_os = "macos")]
const SHORTCUT_CLEAR_SCREEN: &str = "Cmd+Alt+K";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CLEAR_SCREEN: &str = "Ctrl+Alt+K";
#[cfg(target_os = "macos")]
const SHORTCUT_QUIT: &str = "Cmd+Q";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_QUIT: &str = "Ctrl+Shift+Q";

/// Actions that can be triggered from the command palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    TabCreate,
    LocalTabCreate,
    TabClose,
    TabCloseUnfocused,
    SelectNextTab,
    SelectPrevTab,
    SelectNextLocalTab,
    SelectPrevLocalTab,
    SplitRight,
    SplitDown,
    CloneSplitRight,
    CloneSplitDown,
    SelectNextSplit,
    SelectPrevSplit,
    SelectPaneLeft,
    SelectPaneRight,
    SelectPaneUp,
    SelectPaneDown,
    ConfigEditor,
    WindowCreateNew,
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
    ToggleViMode,
    ToggleFullscreen,
    ToggleAppearanceTheme,
    Copy,
    Paste,
    ScrollToPreviousCommand,
    ScrollToNextCommand,
    SearchForward,
    SearchBackward,
    SearchGlobalForward,
    SearchGlobalBackward,
    PreviewSelectedImage,
    ClearScreen,
    CloseCurrentSplitOrTab,
    OpenMarket,
    /// Open the application-owned, read-only Connection Hub. This action
    /// grants no filesystem, network, process, authentication, or PTY access.
    OpenConnections,
    /// Search typed Quick Actions. Selection enters a separate review step;
    /// this action never writes to the PTY itself.
    OpenActions,
    /// Browse the family names of every registered font. Does NOT
    /// execute a one-shot action — the palette stays open with the
    /// font list as its contents. Handled by `router`, not
    /// `Screen::execute_palette_action`.
    ListFonts,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandIcon {
    Back,
    TabAdd,
    LocalTabAdd,
    TabClose,
    TabsClose,
    TabNext,
    TabPrevious,
    SplitRight,
    SplitDown,
    CloneSplitRight,
    CloneSplitDown,
    PaneNext,
    PanePrevious,
    Close,
    Settings,
    WindowAdd,
    FontIncrease,
    FontDecrease,
    FontReset,
    Code,
    Fullscreen,
    Theme,
    Copy,
    Paste,
    Search,
    Image,
    History,
    Connections,
    Extension,
    Font,
    Power,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RowPresentation {
    icon: CommandIcon,
    accent: [f32; 4],
}

fn command_presentation(action: PaletteAction) -> RowPresentation {
    use PaletteAction::*;
    match action {
        TabCreate => RowPresentation {
            icon: CommandIcon::TabAdd,
            accent: BRAND_CYAN,
        },
        LocalTabCreate => RowPresentation {
            icon: CommandIcon::LocalTabAdd,
            accent: BRAND_LIME,
        },
        TabClose => RowPresentation {
            icon: CommandIcon::TabClose,
            accent: BRAND_CORAL,
        },
        TabCloseUnfocused => RowPresentation {
            icon: CommandIcon::TabsClose,
            accent: BRAND_CORAL,
        },
        CloseCurrentSplitOrTab => RowPresentation {
            icon: CommandIcon::Close,
            accent: BRAND_CORAL,
        },
        SelectNextTab => RowPresentation {
            icon: CommandIcon::TabNext,
            accent: BRAND_BLUE,
        },
        SelectPrevTab => RowPresentation {
            icon: CommandIcon::TabPrevious,
            accent: BRAND_BLUE,
        },
        SelectNextLocalTab => RowPresentation {
            icon: CommandIcon::TabNext,
            accent: BRAND_LIME,
        },
        SelectPrevLocalTab => RowPresentation {
            icon: CommandIcon::TabPrevious,
            accent: BRAND_LIME,
        },
        SplitRight => RowPresentation {
            icon: CommandIcon::SplitRight,
            accent: BRAND_PURPLE,
        },
        SplitDown => RowPresentation {
            icon: CommandIcon::SplitDown,
            accent: BRAND_PURPLE,
        },
        CloneSplitRight => RowPresentation {
            icon: CommandIcon::CloneSplitRight,
            accent: BRAND_CYAN,
        },
        CloneSplitDown => RowPresentation {
            icon: CommandIcon::CloneSplitDown,
            accent: BRAND_CYAN,
        },
        SelectNextSplit => RowPresentation {
            icon: CommandIcon::PaneNext,
            accent: BRAND_PURPLE,
        },
        SelectPrevSplit => RowPresentation {
            icon: CommandIcon::PanePrevious,
            accent: BRAND_PURPLE,
        },
        SelectPaneLeft | SelectPaneUp => RowPresentation {
            icon: CommandIcon::PanePrevious,
            accent: BRAND_CYAN,
        },
        SelectPaneRight | SelectPaneDown => RowPresentation {
            icon: CommandIcon::PaneNext,
            accent: BRAND_CYAN,
        },
        ConfigEditor => RowPresentation {
            icon: CommandIcon::Settings,
            accent: BRAND_AMBER,
        },
        WindowCreateNew => RowPresentation {
            icon: CommandIcon::WindowAdd,
            accent: BRAND_CYAN,
        },
        IncreaseFontSize => RowPresentation {
            icon: CommandIcon::FontIncrease,
            accent: BRAND_PURPLE,
        },
        DecreaseFontSize => RowPresentation {
            icon: CommandIcon::FontDecrease,
            accent: BRAND_PURPLE,
        },
        ResetFontSize => RowPresentation {
            icon: CommandIcon::FontReset,
            accent: BRAND_PURPLE,
        },
        ListFonts => RowPresentation {
            icon: CommandIcon::Font,
            accent: BRAND_PURPLE,
        },
        ToggleViMode => RowPresentation {
            icon: CommandIcon::Code,
            accent: BRAND_LIME,
        },
        ToggleFullscreen => RowPresentation {
            icon: CommandIcon::Fullscreen,
            accent: BRAND_BLUE,
        },
        ToggleAppearanceTheme => RowPresentation {
            icon: CommandIcon::Theme,
            accent: BRAND_AMBER,
        },
        Copy => RowPresentation {
            icon: CommandIcon::Copy,
            accent: BRAND_CYAN,
        },
        Paste => RowPresentation {
            icon: CommandIcon::Paste,
            accent: BRAND_CYAN,
        },
        ScrollToPreviousCommand => RowPresentation {
            icon: CommandIcon::History,
            accent: BRAND_CYAN,
        },
        ScrollToNextCommand => RowPresentation {
            icon: CommandIcon::History,
            accent: BRAND_PURPLE,
        },
        SearchForward | SearchBackward => RowPresentation {
            icon: CommandIcon::Search,
            accent: BRAND_BLUE,
        },
        SearchGlobalForward | SearchGlobalBackward => RowPresentation {
            icon: CommandIcon::Search,
            accent: BRAND_PURPLE,
        },
        PreviewSelectedImage => RowPresentation {
            icon: CommandIcon::Image,
            accent: BRAND_CYAN,
        },
        ClearScreen => RowPresentation {
            icon: CommandIcon::History,
            accent: BRAND_AMBER,
        },
        OpenMarket => RowPresentation {
            icon: CommandIcon::Extension,
            accent: BRAND_LIME,
        },
        OpenConnections => RowPresentation {
            icon: CommandIcon::Connections,
            accent: BRAND_BLUE,
        },
        OpenActions => RowPresentation {
            icon: CommandIcon::Code,
            accent: BRAND_CYAN,
        },
        Quit => RowPresentation {
            icon: CommandIcon::Power,
            accent: BRAND_CORAL,
        },
    }
}

struct Command {
    title: &'static str,
    shortcut: &'static str,
    action: PaletteAction,
}

fn palette_binding_target(
    action: PaletteAction,
) -> Option<(&'static str, Option<&'static str>)> {
    use PaletteAction::*;
    match action {
        TabCreate => Some(("new_tab", None)),
        TabClose => Some(("close_tab", Some("this"))),
        SelectNextTab => Some(("next_tab", None)),
        SelectPrevTab => Some(("previous_tab", None)),
        SplitRight => Some(("new_split", Some("right"))),
        SplitDown => Some(("new_split", Some("down"))),
        SelectNextSplit => Some(("goto_split", Some("next"))),
        SelectPrevSplit => Some(("goto_split", Some("previous"))),
        SelectPaneLeft => Some(("goto_split", Some("left"))),
        SelectPaneRight => Some(("goto_split", Some("right"))),
        SelectPaneUp => Some(("goto_split", Some("up"))),
        SelectPaneDown => Some(("goto_split", Some("down"))),
        ConfigEditor => Some(("open_config", None)),
        WindowCreateNew => Some(("new_window", None)),
        IncreaseFontSize => Some(("increase_font_size", Some("1"))),
        DecreaseFontSize => Some(("decrease_font_size", Some("1"))),
        ResetFontSize => Some(("reset_font_size", None)),
        ToggleFullscreen => Some(("toggle_fullscreen", None)),
        Copy => Some(("copy_to_clipboard", None)),
        Paste => Some(("paste_from_clipboard", None)),
        // The typed dispatcher starts forward in one pane; its schema accepts
        // no direction/scope parameter. Do not invent a broader/backward alias.
        SearchForward => Some(("start_search", None)),
        SearchBackward | SearchGlobalForward | SearchGlobalBackward => None,
        ClearScreen => Some(("clear_screen", None)),
        ScrollToPreviousCommand => Some(("jump_to_prompt", Some("-1"))),
        ScrollToNextCommand => Some(("jump_to_prompt", Some("1"))),
        CloseCurrentSplitOrTab => Some(("close_surface", None)),
        Quit => Some(("quit", None)),
        LocalTabCreate
        | TabCloseUnfocused
        | SelectNextLocalTab
        | SelectPrevLocalTab
        | CloneSplitRight
        | CloneSplitDown
        | ToggleViMode
        | ToggleAppearanceTheme
        | PreviewSelectedImage
        | OpenMarket
        | OpenConnections
        | OpenActions
        | ListFonts => None,
    }
}

// Match the action actually executed by the palette, not a similarly named
// operation (ClearHistory, for example, does not clear the visible screen).
fn legacy_binding_target(action: PaletteAction) -> crate::bindings::Action {
    use crate::bindings::Action;
    use PaletteAction::*;
    match action {
        TabCreate => Action::TabCreateNew,
        LocalTabCreate => Action::LocalTabCreateNew,
        TabClose => Action::TabCloseCurrent,
        TabCloseUnfocused => Action::TabCloseUnfocused,
        SelectNextTab => Action::SelectNextTab,
        SelectPrevTab => Action::SelectPrevTab,
        SelectNextLocalTab => Action::SelectNextLocalTab,
        SelectPrevLocalTab => Action::SelectPrevLocalTab,
        SplitRight => Action::SplitRight,
        SplitDown => Action::SplitDown,
        CloneSplitRight => Action::CloneSplitRight,
        CloneSplitDown => Action::CloneSplitDown,
        SelectNextSplit => Action::SelectNextSplit,
        SelectPrevSplit => Action::SelectPrevSplit,
        SelectPaneLeft => Action::SelectPaneLeft,
        SelectPaneRight => Action::SelectPaneRight,
        SelectPaneUp => Action::SelectPaneUp,
        SelectPaneDown => Action::SelectPaneDown,
        CloseCurrentSplitOrTab => Action::CloseCurrentSplitOrTab,
        ConfigEditor => Action::ConfigEditor,
        WindowCreateNew => Action::WindowCreateNew,
        IncreaseFontSize => Action::IncreaseFontSize,
        DecreaseFontSize => Action::DecreaseFontSize,
        ResetFontSize => Action::ResetFontSize,
        ToggleViMode => Action::ToggleViMode,
        ToggleFullscreen => Action::ToggleFullscreen,
        ToggleAppearanceTheme => Action::ToggleAppearanceTheme,
        Copy => Action::Copy,
        Paste => Action::Paste,
        ScrollToPreviousCommand => Action::ScrollToPrevPrompt,
        ScrollToNextCommand => Action::ScrollToNextPrompt,
        SearchForward => Action::SearchForward,
        SearchBackward => Action::SearchBackward,
        SearchGlobalForward => Action::SearchGlobalForward,
        SearchGlobalBackward => Action::SearchGlobalBackward,
        PreviewSelectedImage => Action::PreviewSelectedImage,
        ClearScreen => Action::ClearScreen,
        OpenMarket => Action::OpenExtensionMarketplace,
        OpenConnections => Action::OpenConnectionHub,
        OpenActions => Action::OpenActionCenter,
        ListFonts => Action::OpenFontBrowser,
        Quit => Action::Quit,
    }
}

fn registry_binding_for_command(
    registry: &automexia_keybindings::CompiledRegistry,
    action: PaletteAction,
) -> Option<&automexia_keybindings::CompiledBinding> {
    palette_binding_target(action).and_then(|(id, parameter)| {
        registry.bindings_for_action(id).find(|binding| {
            binding.table == "default"
                && binding.sequence.len() == 1
                && binding
                    .predicate
                    .matches(automexia_keybindings::ModeFlags::empty())
                && binding.scope == automexia_keybindings::BindingScope::FocusedSurface
                && binding.actions.len() == 1
                && parameter.is_none_or(|expected| {
                    binding.actions.iter().any(|action| {
                        action.id.as_str() == id
                            && action.parameter.as_deref() == Some(expected)
                    })
                })
        })
    })
}

const COMMANDS: &[Command] = &[
    Command {
        title: "New Window Tab",
        shortcut: SHORTCUT_NEW_TAB,
        action: PaletteAction::TabCreate,
    },
    Command {
        title: "New Tab in Selected Session",
        shortcut: SHORTCUT_NEW_LOCAL_TAB,
        action: PaletteAction::LocalTabCreate,
    },
    Command {
        title: "Close Tab",
        shortcut: SHORTCUT_CLOSE_TAB,
        action: PaletteAction::TabClose,
    },
    Command {
        title: "Close Other Tabs",
        shortcut: SHORTCUT_CLOSE_OTHER_TABS,
        action: PaletteAction::TabCloseUnfocused,
    },
    Command {
        title: "Next Tab",
        shortcut: "Ctrl+Tab",
        action: PaletteAction::SelectNextTab,
    },
    Command {
        title: "Previous Tab",
        shortcut: "Ctrl+Shift+Tab",
        action: PaletteAction::SelectPrevTab,
    },
    Command {
        title: "Next Tab in Selected Pane",
        shortcut: SHORTCUT_NEXT_LOCAL_TAB,
        action: PaletteAction::SelectNextLocalTab,
    },
    Command {
        title: "Previous Tab in Selected Pane",
        shortcut: SHORTCUT_PREV_LOCAL_TAB,
        action: PaletteAction::SelectPrevLocalTab,
    },
    Command {
        title: "Split Right",
        shortcut: SHORTCUT_SPLIT_RIGHT,
        action: PaletteAction::SplitRight,
    },
    Command {
        title: "Split Down",
        shortcut: SHORTCUT_SPLIT_DOWN,
        action: PaletteAction::SplitDown,
    },
    Command {
        title: "Clone Active Session Right",
        shortcut: SHORTCUT_CLONE_RIGHT,
        action: PaletteAction::CloneSplitRight,
    },
    Command {
        title: "Clone Active Session Down",
        shortcut: SHORTCUT_CLONE_DOWN,
        action: PaletteAction::CloneSplitDown,
    },
    Command {
        title: "Next Split",
        shortcut: SHORTCUT_NEXT_PANE,
        action: PaletteAction::SelectNextSplit,
    },
    Command {
        title: "Previous Split",
        shortcut: SHORTCUT_PREV_PANE,
        action: PaletteAction::SelectPrevSplit,
    },
    Command {
        title: "Focus Pane Left",
        shortcut: SHORTCUT_PANE_LEFT,
        action: PaletteAction::SelectPaneLeft,
    },
    Command {
        title: "Focus Pane Right",
        shortcut: SHORTCUT_PANE_RIGHT,
        action: PaletteAction::SelectPaneRight,
    },
    Command {
        title: "Focus Pane Up",
        shortcut: SHORTCUT_PANE_UP,
        action: PaletteAction::SelectPaneUp,
    },
    Command {
        title: "Focus Pane Down",
        shortcut: SHORTCUT_PANE_DOWN,
        action: PaletteAction::SelectPaneDown,
    },
    Command {
        title: "Close Split or Tab",
        shortcut: SHORTCUT_CLOSE_SURFACE,
        action: PaletteAction::CloseCurrentSplitOrTab,
    },
    Command {
        title: "Settings",
        shortcut: SHORTCUT_SETTINGS,
        action: PaletteAction::ConfigEditor,
    },
    Command {
        title: "New Window",
        shortcut: SHORTCUT_NEW_WINDOW,
        action: PaletteAction::WindowCreateNew,
    },
    Command {
        title: "Increase Font Size",
        shortcut: SHORTCUT_FONT_UP,
        action: PaletteAction::IncreaseFontSize,
    },
    Command {
        title: "Decrease Font Size",
        shortcut: SHORTCUT_FONT_DOWN,
        action: PaletteAction::DecreaseFontSize,
    },
    Command {
        title: "Reset Font Size",
        shortcut: SHORTCUT_FONT_RESET,
        action: PaletteAction::ResetFontSize,
    },
    Command {
        title: "Toggle Vi Mode",
        shortcut: SHORTCUT_VI_MODE,
        action: PaletteAction::ToggleViMode,
    },
    Command {
        title: "Toggle Fullscreen",
        shortcut: SHORTCUT_FULLSCREEN,
        action: PaletteAction::ToggleFullscreen,
    },
    Command {
        title: "Toggle Appearance Theme",
        shortcut: SHORTCUT_APPEARANCE,
        action: PaletteAction::ToggleAppearanceTheme,
    },
    Command {
        title: "Copy",
        shortcut: SHORTCUT_COPY,
        action: PaletteAction::Copy,
    },
    Command {
        title: "Paste",
        shortcut: SHORTCUT_PASTE,
        action: PaletteAction::Paste,
    },
    Command {
        title: "Jump to Previous Command",
        shortcut: SHORTCUT_PREVIOUS_COMMAND,
        action: PaletteAction::ScrollToPreviousCommand,
    },
    Command {
        title: "Jump to Next Command",
        shortcut: SHORTCUT_NEXT_COMMAND,
        action: PaletteAction::ScrollToNextCommand,
    },
    Command {
        title: "Find in Pane",
        shortcut: SHORTCUT_SEARCH,
        action: PaletteAction::SearchForward,
    },
    Command {
        title: "Find Previous in Pane",
        shortcut: SHORTCUT_SEARCH_BACKWARD,
        action: PaletteAction::SearchBackward,
    },
    Command {
        title: "Search All Visible Panes",
        shortcut: SHORTCUT_SEARCH_GLOBAL,
        action: PaletteAction::SearchGlobalForward,
    },
    Command {
        title: "Search All Visible Panes Backward",
        shortcut: SHORTCUT_SEARCH_GLOBAL_BACKWARD,
        action: PaletteAction::SearchGlobalBackward,
    },
    Command {
        title: "Preview Selected Image",
        shortcut: SHORTCUT_PREVIEW_IMAGE,
        action: PaletteAction::PreviewSelectedImage,
    },
    Command {
        title: "Clear Screen and History",
        shortcut: SHORTCUT_CLEAR_SCREEN,
        action: PaletteAction::ClearScreen,
    },
    Command {
        title: "Connection Hub (read-only)",
        shortcut: SHORTCUT_CONNECTION_HUB,
        action: PaletteAction::OpenConnections,
    },
    Command {
        title: "Quick Actions",
        shortcut: SHORTCUT_QUICK_ACTIONS,
        action: PaletteAction::OpenActions,
    },
    Command {
        title: "Extensions",
        shortcut: SHORTCUT_EXTENSIONS,
        action: PaletteAction::OpenMarket,
    },
    Command {
        title: "List Fonts",
        shortcut: SHORTCUT_FONT_BROWSER,
        action: PaletteAction::ListFonts,
    },
    Command {
        title: "Quit",
        shortcut: SHORTCUT_QUIT,
        action: PaletteAction::Quit,
    },
];

/// What the palette is currently browsing and filtering over.
///
/// `Commands` is the default — fuzzy-matches against the static
/// `COMMANDS` list and dispatches a `PaletteAction` on Enter.
///
/// `Fonts` is entered via the `ListFonts` command. The palette stays
/// open, its content is replaced with the owned list of font family
/// names, and Enter closes the palette (no font-switching action yet).
/// The list is owned so the filter pass doesn't keep a borrow on the
/// sugarloaf FontLibrary.
enum PaletteMode {
    Commands,
    Fonts(Vec<String>),
    Market(Vec<MarketItem>),
    QuickActions {
        items: Vec<QuickActionListItem>,
        notice: String,
    },
    QuickActionPlaceholder {
        prompt: String,
    },
    QuickActionReview(QuickActionReviewView),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickActionReviewChoice {
    Insert,
    Copy,
}

/// One row in the filtered result list. Variants carry exactly the
/// data the render pass needs — no `&'static Command` vs `&str`
/// lifetime mixing.
enum PaletteRow<'a> {
    Navigation(Option<Category>),
    Command {
        title: &'a str,
        shortcut: &'a str,
        action: PaletteAction,
    },
    Font {
        family: &'a str,
    },
    Market {
        id: &'a str,
        name: &'a str,
        installed: bool,
    },
    QuickAction {
        item: &'a QuickActionListItem,
    },
    QuickActionNotice {
        message: &'a str,
    },
    PlaceholderContinue,
    ReviewCommand {
        command: &'a str,
        context: &'a str,
    },
    ReviewInsert {
        label: &'a str,
        risk: QuickActionRisk,
    },
    ReviewCopy {
        risk: QuickActionRisk,
    },
}

impl<'a> PaletteRow<'a> {
    fn title(&self) -> &'a str {
        match *self {
            PaletteRow::Navigation(Some(category)) => category.title(),
            PaletteRow::Navigation(None) => "Back to categories",
            PaletteRow::Command { title, .. } => title,
            PaletteRow::Font { family } => family,
            PaletteRow::Market { name, .. } => name,
            PaletteRow::QuickAction { item } => &item.name,
            PaletteRow::QuickActionNotice { message } => message,
            PaletteRow::PlaceholderContinue => "Continue to review",
            PaletteRow::ReviewCommand { command, .. } => command,
            PaletteRow::ReviewInsert { label, .. } => label,
            PaletteRow::ReviewCopy { .. } => "Copy command",
        }
    }

    fn shortcut(&self) -> &'a str {
        match *self {
            PaletteRow::Navigation(Some(_)) => "Enter ›",
            PaletteRow::Navigation(None) => "Alt+Left",
            PaletteRow::Command { shortcut, .. } => shortcut,
            PaletteRow::Font { .. } => "",
            PaletteRow::Market {
                installed: true, ..
            } => "Remove",
            PaletteRow::Market {
                installed: false, ..
            } => "Install",
            PaletteRow::QuickAction { item } => item.metadata_label.as_str(),
            PaletteRow::QuickActionNotice { .. } => "",
            PaletteRow::PlaceholderContinue => "Enter",
            PaletteRow::ReviewCommand { context, .. } => context,
            PaletteRow::ReviewInsert { risk, .. } | PaletteRow::ReviewCopy { risk } => {
                risk_label(risk)
            }
        }
    }

    fn action(&self) -> Option<PaletteAction> {
        match *self {
            PaletteRow::Navigation(_) => None,
            PaletteRow::Command { action, .. } => Some(action),
            PaletteRow::Font { .. }
            | PaletteRow::Market { .. }
            | PaletteRow::QuickAction { .. }
            | PaletteRow::QuickActionNotice { .. }
            | PaletteRow::PlaceholderContinue
            | PaletteRow::ReviewCommand { .. }
            | PaletteRow::ReviewInsert { .. }
            | PaletteRow::ReviewCopy { .. } => None,
        }
    }

    fn presentation(&self) -> RowPresentation {
        match *self {
            PaletteRow::Navigation(Some(category)) => category.presentation(),
            PaletteRow::Navigation(None) => RowPresentation {
                icon: CommandIcon::Back,
                accent: BRAND_CYAN,
            },
            PaletteRow::Command { action, .. } => command_presentation(action),
            PaletteRow::Font { .. } => RowPresentation {
                icon: CommandIcon::Font,
                accent: BRAND_PURPLE,
            },
            PaletteRow::Market {
                installed: true, ..
            } => RowPresentation {
                icon: CommandIcon::Extension,
                accent: BRAND_LIME,
            },
            PaletteRow::Market {
                installed: false, ..
            } => RowPresentation {
                icon: CommandIcon::Extension,
                accent: BRAND_CYAN,
            },
            PaletteRow::QuickAction { item } if item.provider_context => {
                RowPresentation {
                    icon: CommandIcon::Connections,
                    accent: BRAND_CYAN,
                }
            }
            PaletteRow::QuickAction { item } => RowPresentation {
                icon: CommandIcon::Code,
                accent: risk_accent(item.risk),
            },
            PaletteRow::QuickActionNotice { .. } => RowPresentation {
                icon: CommandIcon::History,
                accent: BRAND_BLUE,
            },
            PaletteRow::PlaceholderContinue => RowPresentation {
                icon: CommandIcon::TabNext,
                accent: BRAND_CYAN,
            },
            PaletteRow::ReviewCommand { .. } => RowPresentation {
                icon: CommandIcon::Code,
                accent: BRAND_BLUE,
            },
            PaletteRow::ReviewInsert { risk, .. } => RowPresentation {
                icon: CommandIcon::Paste,
                accent: risk_accent(risk),
            },
            PaletteRow::ReviewCopy { risk } => RowPresentation {
                icon: CommandIcon::Copy,
                accent: risk_accent(risk),
            },
        }
    }
}

const fn risk_label(risk: QuickActionRisk) -> &'static str {
    match risk {
        QuickActionRisk::ReadOnly => "Read-only",
        QuickActionRisk::Mutating => "Mutating",
        QuickActionRisk::Destructive => "Destructive",
        QuickActionRisk::Privileged => "Privileged",
    }
}

const fn risk_accent(risk: QuickActionRisk) -> [f32; 4] {
    match risk {
        QuickActionRisk::ReadOnly => BRAND_LIME,
        QuickActionRisk::Mutating => BRAND_AMBER,
        QuickActionRisk::Destructive | QuickActionRisk::Privileged => BRAND_CORAL,
    }
}

/// Paint a rounded-rect outline by layering two filled rounded rects:
/// the outer one in `stroke_color`, then a smaller one in `fill_color`
/// inset by `stroke` on all sides to carve out the interior. Sugarloaf
/// has no stroked-rect primitive, so this is how we get a 1px border
/// effect. Nine params is the irreducible minimum here — grouping them
/// into a struct would just shuffle the same fields.
#[allow(clippy::too_many_arguments)]
fn stroke_rounded_rect(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    stroke: f32,
    radius: f32,
    stroke_color: [f32; 4],
    fill_color: [f32; 4],
    depth: f32,
    order: u8,
) {
    sugarloaf.rounded_rect(
        None,
        x,
        y,
        width,
        height,
        stroke_color,
        depth,
        radius,
        order,
    );
    let inner_radius = (radius - stroke).max(0.0);
    // Inset fill carves out the interior. Painted slightly deeper so
    // it lands on top of the outer rect.
    sugarloaf.rounded_rect(
        None,
        x + stroke,
        y + stroke,
        (width - stroke * 2.0).max(0.0),
        (height - stroke * 2.0).max(0.0),
        fill_color,
        depth + 0.001,
        inner_radius,
        order,
    );
}

/// Paint a "copy" icon (two overlapping rounded page outlines)
/// anchored at `(x, y)`. Drawn from rects only — no font glyph
/// dependency — so it renders consistently regardless of what the
/// user's font stack can produce for ⎘ / 📋 / similar.
///
/// `row_fill_color` is the background behind the icon (palette BG when
/// the row is idle, selection highlight when hovered/selected); it's
/// used to cut out the page interiors so the outlines read as a
/// proper border rather than two solid blobs. Back page painted
/// slightly below the front via depth so the front's cutout
/// correctly hides the overlapping portion of the back's stroke.
#[allow(clippy::too_many_arguments)]
fn draw_copy_icon(
    sugarloaf: &mut Sugarloaf,
    x: f32,
    y: f32,
    stroke_color: [f32; 4],
    row_fill_color: [f32; 4],
    depth: f32,
    order: u8,
) {
    // Back page (upper-left).
    stroke_rounded_rect(
        sugarloaf,
        x,
        y,
        COPY_ICON_PAGE_W,
        COPY_ICON_PAGE_H,
        COPY_ICON_STROKE,
        COPY_ICON_RADIUS,
        stroke_color,
        row_fill_color,
        depth,
        order,
    );
    // Front page (offset down-right), painted above the back so its
    // cutout hides the back's overlapping interior.
    stroke_rounded_rect(
        sugarloaf,
        x + COPY_ICON_OFFSET,
        y + COPY_ICON_OFFSET,
        COPY_ICON_PAGE_W,
        COPY_ICON_PAGE_H,
        COPY_ICON_STROKE,
        COPY_ICON_RADIUS,
        stroke_color,
        row_fill_color,
        depth + 0.01,
        order,
    );
}

/// Small vector canvas shared by every palette icon. Keeping all actions on a
/// 22 px grid with the same stroke weight avoids font fallback, baseline drift,
/// and the mismatched optical sizes of icon-font glyphs.
struct IconCanvas<'a, 'font> {
    sugarloaf: &'a mut Sugarloaf<'font>,
    x: f32,
    y: f32,
    color: [f32; 4],
    fill: [f32; 4],
    depth: f32,
    order: u8,
}

impl IconCanvas<'_, '_> {
    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.sugarloaf.line(
            self.x + x1,
            self.y + y1,
            self.x + x2,
            self.y + y2,
            1.45,
            self.depth,
            self.color,
            self.order,
        );
    }

    fn outline(&mut self, x: f32, y: f32, width: f32, height: f32, radius: f32) {
        stroke_rounded_rect(
            self.sugarloaf,
            self.x + x,
            self.y + y,
            width,
            height,
            1.25,
            radius,
            self.color,
            self.fill,
            self.depth,
            self.order,
        );
    }

    fn dot(&mut self, x: f32, y: f32, size: f32) {
        self.sugarloaf.rounded_rect(
            None,
            self.x + x,
            self.y + y,
            size,
            size,
            self.color,
            self.depth + 0.01,
            size / 2.0,
            self.order,
        );
    }

    fn plus(&mut self, x: f32, y: f32, radius: f32) {
        self.line(x - radius, y, x + radius, y);
        self.line(x, y - radius, x, y + radius);
    }

    fn chevron_right(&mut self, x: f32, y: f32, radius: f32) {
        self.line(x - radius, y - radius, x, y);
        self.line(x, y, x - radius, y + radius);
    }

    fn chevron_left(&mut self, x: f32, y: f32, radius: f32) {
        self.line(x + radius, y - radius, x, y);
        self.line(x, y, x + radius, y + radius);
    }

    fn tab_frame(&mut self) {
        self.outline(1.5, 3.0, 19.0, 16.0, 3.5);
        self.line(5.0, 6.0, 11.0, 6.0);
    }

    fn font_mark(&mut self) {
        self.line(3.5, 18.5, 9.0, 4.0);
        self.line(9.0, 4.0, 14.5, 18.5);
        self.line(5.8, 12.5, 12.2, 12.5);
    }
}

// The same three strokes serve the fixed header and the navigable Back row.
const BACK_ARROW_STROKES: [[f32; 4]; 3] = [
    [4.0, 11.0, 18.0, 11.0],
    [4.0, 11.0, 10.0, 5.0],
    [4.0, 11.0, 10.0, 17.0],
];

fn draw_command_icon(
    sugarloaf: &mut Sugarloaf,
    icon: CommandIcon,
    x: f32,
    y: f32,
    color: [f32; 4],
    fill: [f32; 4],
) {
    let mut canvas = IconCanvas {
        sugarloaf,
        x,
        y,
        color,
        fill,
        depth: DEPTH_ELEMENT + 0.035,
        order: ORDER,
    };

    match icon {
        CommandIcon::Back => {
            for [x1, y1, x2, y2] in BACK_ARROW_STROKES {
                canvas.line(x1, y1, x2, y2);
            }
        }
        CommandIcon::TabAdd => {
            canvas.tab_frame();
            canvas.plus(15.5, 12.5, 2.7);
        }
        CommandIcon::LocalTabAdd => {
            canvas.outline(1.5, 3.0, 19.0, 16.0, 3.5);
            canvas.line(8.5, 3.5, 8.5, 18.5);
            canvas.plus(15.0, 11.0, 2.8);
        }
        CommandIcon::TabClose => {
            canvas.tab_frame();
            canvas.line(12.5, 12.5, 18.5, 12.5);
        }
        CommandIcon::TabsClose => {
            canvas.outline(4.0, 1.5, 16.5, 14.0, 3.0);
            canvas.outline(1.5, 5.0, 16.5, 14.0, 3.0);
            canvas.line(10.0, 13.5, 16.0, 13.5);
        }
        CommandIcon::TabNext => {
            canvas.tab_frame();
            canvas.chevron_right(15.5, 12.5, 3.0);
        }
        CommandIcon::TabPrevious => {
            canvas.tab_frame();
            canvas.chevron_left(7.0, 12.5, 3.0);
        }
        CommandIcon::SplitRight => {
            canvas.outline(1.5, 2.0, 19.0, 18.0, 3.5);
            canvas.line(11.0, 2.5, 11.0, 19.5);
            canvas.chevron_right(16.0, 11.0, 2.5);
        }
        CommandIcon::SplitDown => {
            canvas.outline(1.5, 2.0, 19.0, 18.0, 3.5);
            canvas.line(2.0, 11.0, 20.0, 11.0);
            canvas.line(8.5, 15.0, 11.0, 17.5);
            canvas.line(11.0, 17.5, 13.5, 15.0);
        }
        CommandIcon::CloneSplitRight => {
            canvas.outline(1.5, 1.5, 15.0, 14.0, 3.0);
            canvas.outline(5.5, 6.5, 15.0, 14.0, 3.0);
            canvas.line(13.0, 7.0, 13.0, 20.0);
            canvas.chevron_right(17.0, 13.5, 2.2);
        }
        CommandIcon::CloneSplitDown => {
            canvas.outline(1.5, 1.5, 15.0, 14.0, 3.0);
            canvas.outline(5.5, 6.5, 15.0, 14.0, 3.0);
            canvas.line(6.0, 13.5, 20.0, 13.5);
            canvas.line(10.5, 16.0, 13.0, 18.5);
            canvas.line(13.0, 18.5, 15.5, 16.0);
        }
        CommandIcon::PaneNext => {
            canvas.outline(1.5, 2.0, 19.0, 18.0, 3.5);
            canvas.line(11.0, 2.5, 11.0, 19.5);
            canvas.chevron_right(17.0, 11.0, 2.5);
        }
        CommandIcon::PanePrevious => {
            canvas.outline(1.5, 2.0, 19.0, 18.0, 3.5);
            canvas.line(11.0, 2.5, 11.0, 19.5);
            canvas.chevron_left(5.0, 11.0, 2.5);
        }
        CommandIcon::Close => {
            canvas.outline(3.5, 3.5, 15.0, 15.0, 7.5);
            canvas.line(7.0, 11.0, 15.0, 11.0);
        }
        CommandIcon::Settings => {
            canvas.line(2.5, 5.0, 19.5, 5.0);
            canvas.line(2.5, 11.0, 19.5, 11.0);
            canvas.line(2.5, 17.0, 19.5, 17.0);
            canvas.dot(6.0, 3.25, 3.5);
            canvas.dot(13.0, 9.25, 3.5);
            canvas.dot(8.5, 15.25, 3.5);
        }
        CommandIcon::WindowAdd => {
            canvas.outline(1.5, 2.5, 19.0, 17.0, 3.5);
            canvas.line(2.5, 6.5, 19.5, 6.5);
            canvas.plus(15.0, 13.0, 3.0);
        }
        CommandIcon::FontIncrease => {
            canvas.font_mark();
            canvas.plus(18.0, 6.0, 2.8);
        }
        CommandIcon::FontDecrease => {
            canvas.font_mark();
            canvas.line(15.2, 6.0, 20.8, 6.0);
        }
        CommandIcon::FontReset => {
            canvas.font_mark();
            canvas.line(15.0, 5.0, 20.0, 5.0);
            canvas.line(15.0, 5.0, 17.0, 3.0);
            canvas.line(15.0, 5.0, 17.0, 7.0);
        }
        CommandIcon::Code => {
            canvas.line(8.0, 5.0, 3.0, 11.0);
            canvas.line(3.0, 11.0, 8.0, 17.0);
            canvas.line(14.0, 5.0, 19.0, 11.0);
            canvas.line(19.0, 11.0, 14.0, 17.0);
        }
        CommandIcon::Fullscreen => {
            canvas.line(2.5, 8.0, 2.5, 2.5);
            canvas.line(2.5, 2.5, 8.0, 2.5);
            canvas.line(14.0, 2.5, 19.5, 2.5);
            canvas.line(19.5, 2.5, 19.5, 8.0);
            canvas.line(19.5, 14.0, 19.5, 19.5);
            canvas.line(19.5, 19.5, 14.0, 19.5);
            canvas.line(8.0, 19.5, 2.5, 19.5);
            canvas.line(2.5, 19.5, 2.5, 14.0);
        }
        CommandIcon::Theme => {
            canvas.outline(7.0, 7.0, 8.0, 8.0, 4.0);
            for (x1, y1, x2, y2) in [
                (11.0, 1.5, 11.0, 4.0),
                (11.0, 18.0, 11.0, 20.5),
                (1.5, 11.0, 4.0, 11.0),
                (18.0, 11.0, 20.5, 11.0),
                (4.2, 4.2, 6.0, 6.0),
                (16.0, 16.0, 17.8, 17.8),
                (16.0, 6.0, 17.8, 4.2),
                (4.2, 17.8, 6.0, 16.0),
            ] {
                canvas.line(x1, y1, x2, y2);
            }
        }
        CommandIcon::Copy => draw_copy_icon(
            canvas.sugarloaf,
            x + 3.0,
            y + 3.0,
            color,
            fill,
            DEPTH_ELEMENT + 0.035,
            ORDER,
        ),
        CommandIcon::Paste => {
            canvas.outline(3.5, 3.5, 15.0, 17.0, 3.0);
            canvas.outline(7.0, 1.5, 8.0, 4.5, 2.0);
            canvas.line(7.0, 10.0, 15.0, 10.0);
            canvas.line(7.0, 14.0, 13.0, 14.0);
        }
        CommandIcon::Search => {
            canvas.outline(2.5, 2.5, 12.5, 12.5, 6.25);
            canvas.line(14.0, 14.0, 20.0, 20.0);
        }
        CommandIcon::Image => {
            canvas.outline(2.0, 3.0, 18.0, 16.0, 3.0);
            canvas.dot(14.5, 6.0, 2.0);
            canvas.line(4.5, 16.0, 9.0, 11.0);
            canvas.line(9.0, 11.0, 12.0, 14.0);
            canvas.line(12.0, 14.0, 15.0, 10.5);
            canvas.line(15.0, 10.5, 19.0, 15.0);
        }
        CommandIcon::History => {
            canvas.outline(2.0, 2.0, 18.0, 18.0, 9.0);
            canvas.line(11.0, 6.0, 11.0, 11.0);
            canvas.line(11.0, 11.0, 15.0, 13.5);
            canvas.chevron_left(2.5, 6.0, 2.0);
        }
        CommandIcon::Connections => {
            canvas.outline(2.0, 3.0, 18.0, 6.0, 2.5);
            canvas.dot(5.0, 5.0, 2.0);
            canvas.line(9.0, 6.0, 17.0, 6.0);
            canvas.outline(2.0, 13.0, 18.0, 6.0, 2.5);
            canvas.dot(5.0, 15.0, 2.0);
            canvas.line(9.0, 16.0, 17.0, 16.0);
            canvas.line(11.0, 9.0, 11.0, 13.0);
        }
        CommandIcon::Extension => {
            canvas.outline(5.0, 5.0, 12.0, 12.0, 3.0);
            canvas.line(8.0, 2.0, 8.0, 5.0);
            canvas.line(14.0, 2.0, 14.0, 5.0);
            canvas.line(8.0, 17.0, 8.0, 20.0);
            canvas.line(14.0, 17.0, 14.0, 20.0);
            canvas.line(2.0, 8.0, 5.0, 8.0);
            canvas.line(17.0, 14.0, 20.0, 14.0);
        }
        CommandIcon::Font => canvas.font_mark(),
        CommandIcon::Power => {
            canvas.outline(2.5, 2.5, 17.0, 17.0, 8.5);
            canvas.line(11.0, 1.0, 11.0, 10.5);
        }
    }
}

/// Fuzzy match: checks if all query chars appear in order in the target.
/// Returns a score (higher = better match), or None if no match.
#[cfg(test)]
fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    fuzzy_score_lowered(&query.to_lowercase(), target)
}

fn fuzzy_score_lowered(query_lower: &str, target: &str) -> Option<i32> {
    if query_lower.is_empty() {
        return Some(0);
    }

    // Empty browsing allocates nothing. ASCII catalogs need no target copy;
    // Unicode retains str::to_lowercase's contextual casing (e.g. final sigma).
    let target_lower = if target.is_ascii() {
        std::borrow::Cow::Borrowed(target)
    } else {
        std::borrow::Cow::Owned(target.to_lowercase())
    };
    let mut query = query_lower.chars().peekable();
    let mut score: i32 = 0;
    let mut prev_match = false;
    let mut first_match_pos = None;
    let mut previous = None;

    for (ti, tc) in target_lower
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .enumerate()
    {
        if query.peek() == Some(&tc) {
            if first_match_pos.is_none() {
                first_match_pos = Some(ti);
            }
            // Consecutive match bonus
            if prev_match {
                score += 5;
            }
            // Word boundary bonus (start of string or after space/punctuation)
            if previous.is_none_or(|c: char| !c.is_alphanumeric()) {
                score += 10;
            }
            prev_match = true;
            query.next();
        } else {
            prev_match = false;
        }
        previous = Some(tc);
    }

    if query.peek().is_some() {
        return None; // Not all query chars matched
    }

    // Bonus for matching near the start
    if let Some(pos) = first_match_pos {
        score += (20_i32).saturating_sub(pos as i32);
    }

    Some(score)
}

/// Command palette UI component (Raycast-style)
pub struct CommandPalette {
    enabled: bool,
    pub query: String,
    pub selected_index: usize,
    scroll_offset: usize,
    pub has_adaptive_theme: bool,
    /// Effective labels built at construction/reload, never on the input path.
    /// The immutable typed registry and legacy adapter remain the only owners.
    registry_shortcuts: Vec<(PaletteAction, String)>,
    /// Which list the palette is showing (commands or fonts).
    mode: PaletteMode,
    category: Option<Category>,
    /// Timestamp for caret blinking
    caret_blink_start: Instant,
    /// Timestamp of the last event that actually changed `scroll_offset`.
    /// Drives the scrollbar's active-to-idle fade while overflow remains
    /// discoverable at a subdued baseline opacity.
    last_scroll_time: Option<Instant>,
    /// Fractional vertical wheel/trackpad motion in logical pixels. A
    /// palette owns this independently from terminal and pane scroll state.
    wheel_accumulated_y: f64,
    /// Number of rows that fit the most recently rendered viewport.
    visible_results: usize,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self {
            enabled: false,
            query: String::new(),
            selected_index: 0,
            scroll_offset: 0,
            has_adaptive_theme: false,
            registry_shortcuts: Vec::new(),
            mode: PaletteMode::Commands,
            category: None,
            caret_blink_start: Instant::now(),
            last_scroll_time: None,
            wheel_accumulated_y: 0.0,
            visible_results: MAX_VISIBLE_RESULTS,
        }
    }
}

impl CommandPalette {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_binding_registry(
        &mut self,
        registry: Option<&automexia_keybindings::CompiledRegistry>,
        profile: automexia_keybindings::ProfileId,
        legacy_unbinds: &[String],
    ) {
        self.registry_shortcuts.clear();
        let Some(registry) = registry else {
            return;
        };
        let strict_profile = profile != automexia_keybindings::ProfileId::Automexia;
        for command in COMMANDS {
            let binding = registry_binding_for_command(registry, command.action);
            let label = if let Some(binding) = binding {
                binding.trigger_label()
            } else if strict_profile
                || legacy_unbinds
                    .iter()
                    .any(|trigger| trigger.eq_ignore_ascii_case(command.shortcut))
            {
                "Unbound".to_string()
            } else {
                continue;
            };
            self.registry_shortcuts.push((command.action, label));
        }
    }

    fn command_shortcut<'a>(&'a self, command: &'a Command) -> &'a str {
        let shortcut = self
            .registry_shortcuts
            .iter()
            .find_map(|(action, shortcut)| {
                (*action == command.action).then_some(shortcut.as_str())
            })
            .unwrap_or(command.shortcut);
        // These actions remain selectable in the palette. Enter describes that
        // local activation, not a fabricated global chord or a restored unbind.
        match shortcut {
            "Unbound" | "Conditional binding" | "Custom binding" => "Enter",
            shortcut => shortcut,
        }
    }

    /// Follow `set_binding_registry` with all effective legacy mappings, so
    /// disabled defaults and tombstones cannot leave guessed shortcut labels.
    pub fn set_effective_bindings(
        &mut self,
        bindings: &[crate::bindings::KeyBinding],
        snapshot: Option<&crate::bindings::registry::RegistrySnapshot>,
    ) {
        use crate::bindings::BindingMode;
        use automexia_keybindings::{ModeFlags, SequenceResolution, SurfaceBindingState};
        for command in COMMANDS {
            let action = command.action;
            let legacy_action = legacy_binding_target(action);
            // Typed profile/user labels already came from the same registry
            // lookup. Legacy fallbacks must not replace that authority.
            if snapshot.is_some_and(|snapshot| {
                registry_binding_for_command(&snapshot.registry, action).is_some()
            }) {
                continue;
            }
            let mut label = "Unbound".to_string();
            for binding in bindings.iter().filter(|binding| {
                binding.action == legacy_action && binding.mode.is_empty()
            }) {
                if !binding.is_triggered_by(
                    BindingMode::empty(),
                    binding.mods,
                    &binding.trigger,
                ) {
                    continue;
                }
                let Some(trigger) = crate::bindings::registry::legacy_trigger(binding)
                else {
                    label = "Custom binding".into();
                    continue;
                };
                if let Some(snapshot) = snapshot {
                    if snapshot.suppresses_legacy_trigger(&trigger, ModeFlags::empty()) {
                        continue;
                    }
                    let mut state = SurfaceBindingState::default();
                    if state.resolve(
                        &snapshot.registry,
                        &trigger,
                        &[],
                        ModeFlags::empty(),
                    ) != SequenceResolution::NoMatch
                    {
                        // Whether a typed action performs can depend on live
                        // selection/topology; do not advertise a shadowed chord.
                        label = "Conditional binding".into();
                        continue;
                    }
                }
                label = trigger.to_string();
                // Dedicated Copy/Paste keys are rare. Prefer an ordinary chord
                // when available, but retain the hardware key as a fallback.
                if matches!(
                    trigger.key,
                    automexia_keybindings::KeyAtom::Named(
                        automexia_keybindings::NamedKey::Copy
                            | automexia_keybindings::NamedKey::Paste
                    )
                ) {
                    continue;
                }
                break;
            }
            self.registry_shortcuts
                .retain(|(candidate, _)| *candidate != action);
            self.registry_shortcuts.push((action, label));
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            self.query.clear();
            self.selected_index = 0;
            self.scroll_offset = 0;
            self.caret_blink_start = Instant::now();
            // Clear scrollbar history so reopening the palette never
            // flashes a leftover scrollbar from the previous session.
            self.last_scroll_time = None;
            self.wheel_accumulated_y = 0.0;
            // Always re-open into Commands mode — a stale Fonts list
            // from a previous session would be misleading (fonts may
            // have changed) and surprising (user toggles palette and
            // finds themselves on the font list).
            self.mode = PaletteMode::Commands;
            self.category = None;
        }
    }

    /// Swap the palette into font-browsing mode with the given family
    /// list. Clears the query so the full list is visible, keeps the
    /// palette open. Called by the router after the user picks the
    /// `List Fonts` command.
    pub fn enter_fonts_mode(&mut self, fonts: Vec<String>) {
        self.mode = PaletteMode::Fonts(fonts);
        self.query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.caret_blink_start = Instant::now();
        self.last_scroll_time = None;
        self.wheel_accumulated_y = 0.0;
    }

    pub fn enter_market_mode(&mut self, items: Vec<MarketItem>) {
        self.mode = PaletteMode::Market(items);
        self.query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.caret_blink_start = Instant::now();
        self.last_scroll_time = None;
        self.wheel_accumulated_y = 0.0;
    }

    pub fn enter_action_search(
        &mut self,
        items: Vec<QuickActionListItem>,
        query: String,
    ) {
        self.mode = PaletteMode::QuickActions {
            items,
            notice: "Loading Quick Actions…".into(),
        };
        self.query = query;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.caret_blink_start = Instant::now();
        self.last_scroll_time = None;
        self.wheel_accumulated_y = 0.0;
    }

    pub fn update_action_items(
        &mut self,
        items: Vec<QuickActionListItem>,
        notice: String,
    ) {
        if matches!(self.mode, PaletteMode::QuickActions { .. }) {
            self.mode = PaletteMode::QuickActions { items, notice };
            self.selected_index = self
                .selected_index
                .min(self.filtered_rows().len().saturating_sub(1));
            self.scroll_offset = self.scroll_offset.min(self.selected_index);
            self.wheel_accumulated_y = 0.0;
        }
    }

    pub fn enter_action_placeholder(&mut self, prompt: String) {
        self.mode = PaletteMode::QuickActionPlaceholder { prompt };
        self.query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.caret_blink_start = Instant::now();
        self.last_scroll_time = None;
        self.wheel_accumulated_y = 0.0;
    }

    pub fn enter_action_review(&mut self, view: QuickActionReviewView) {
        let has_operation = view.copy_allowed;
        self.mode = PaletteMode::QuickActionReview(view);
        self.query.clear();
        // Row zero is the exact, non-actionable command preview. Focus the
        // primary insert/copy operation while keeping the preview reachable.
        self.selected_index = usize::from(has_operation);
        self.scroll_offset = 0;
        self.caret_blink_start = Instant::now();
        self.last_scroll_time = None;
        self.wheel_accumulated_y = 0.0;
    }

    pub fn is_action_search(&self) -> bool {
        matches!(self.mode, PaletteMode::QuickActions { .. })
    }

    pub fn is_action_placeholder(&self) -> bool {
        matches!(self.mode, PaletteMode::QuickActionPlaceholder { .. })
    }

    pub fn is_action_review(&self) -> bool {
        matches!(self.mode, PaletteMode::QuickActionReview(_))
    }

    pub fn set_query(&mut self, query: String) {
        if query.len() > MAX_PALETTE_QUERY_BYTES || query.chars().any(char::is_control) {
            return;
        }
        self.query = query;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.caret_blink_start = Instant::now();
        // Typing reshapes the list entirely — drop any scrollbar
        // fade state so the next scroll starts with a clean timer.
        self.last_scroll_time = None;
        self.wheel_accumulated_y = 0.0;
    }

    /// Reset fractional motion at native gesture boundaries. This never
    /// changes the list position and therefore never wakes the renderer.
    pub fn reset_scroll_gesture(&mut self) {
        self.wheel_accumulated_y = 0.0;
    }

    /// Apply a mouse-wheel delta measured in rows. Positive values move
    /// toward the top, matching winit and terminal scrollback direction.
    pub fn scroll_line_delta(&mut self, lines_y: f32) -> bool {
        if !lines_y.is_finite() {
            return false;
        }
        self.scroll_pixel_delta(
            (lines_y as f64).clamp(-MAX_WHEEL_ROWS_PER_EVENT, MAX_WHEEL_ROWS_PER_EVENT)
                * RESULT_ITEM_HEIGHT as f64,
        )
    }

    /// Apply smooth trackpad motion measured in logical pixels. Partial rows
    /// accumulate deterministically; reversing direction starts fresh so the
    /// user never has to cancel stale momentum before movement is visible.
    pub fn scroll_pixel_delta(&mut self, pixels_y: f64) -> bool {
        if !pixels_y.is_finite() || pixels_y == 0.0 {
            return false;
        }
        let max_pixels = MAX_WHEEL_ROWS_PER_EVENT * RESULT_ITEM_HEIGHT as f64;
        let pixels_y = pixels_y.clamp(-max_pixels, max_pixels);
        if self.wheel_accumulated_y != 0.0
            && self.wheel_accumulated_y.signum() != pixels_y.signum()
        {
            self.wheel_accumulated_y = 0.0;
        }
        self.wheel_accumulated_y =
            (self.wheel_accumulated_y + pixels_y).clamp(-max_pixels, max_pixels);

        let rows_toward_top =
            (self.wheel_accumulated_y / RESULT_ITEM_HEIGHT as f64).trunc() as isize;
        if rows_toward_top == 0 {
            return false;
        }
        self.wheel_accumulated_y %= RESULT_ITEM_HEIGHT as f64;
        self.scroll_rows(rows_toward_top.saturating_neg())
    }

    /// Move the visible window by signed rows (`+` toward the end). The
    /// keyboard selection is clamped into the resulting viewport so Enter
    /// can never activate an invisible command.
    fn scroll_rows(&mut self, rows_toward_end: isize) -> bool {
        let total = self.filtered_rows().len();
        let visible = self.visible_results.max(1);
        let max_offset = total.saturating_sub(visible);
        if max_offset == 0 {
            self.scroll_offset = 0;
            self.selected_index = self.selected_index.min(total.saturating_sub(1));
            self.wheel_accumulated_y = 0.0;
            return false;
        }

        let target = (self.scroll_offset as i128 + rows_toward_end as i128)
            .clamp(0, max_offset as i128) as usize;
        if target == self.scroll_offset {
            self.wheel_accumulated_y = 0.0;
            return false;
        }

        self.scroll_offset = target;
        let last_visible = (target + visible - 1).min(total - 1);
        self.selected_index = self.selected_index.clamp(target, last_visible);
        self.last_scroll_time = Some(Instant::now());
        true
    }

    fn scrollbar_opacity(&self) -> f32 {
        scrollbar::opacity_from_last_scroll(self.last_scroll_time, false)
            .max(PALETTE_SCROLLBAR_IDLE_OPACITY)
    }

    /// Privacy-safe native evidence: positions and counts only, never query or
    /// command text. Available solely in feature-gated GUI test builds.
    #[cfg(feature = "native-gui-test-hooks")]
    pub fn native_test_scroll_state(&self) -> (usize, usize, usize, usize) {
        (
            self.scroll_offset,
            self.selected_index,
            self.visible_results,
            self.filtered_rows().len(),
        )
    }

    /// Public catalog semantics only; never export a user's query or font,
    /// extension, provider, or command-preview content through native evidence.
    #[cfg(any(test, feature = "native-gui-test-hooks"))]
    pub fn accessibility_summary(&self) -> Option<String> {
        self.enabled.then(|| {
            let scope = if matches!(self.mode, PaletteMode::Commands) {
                if !self.query.is_empty() { "All commands" }
                else { self.category.map_or("Command categories", Category::title) }
            } else { "Items" };
            let count = self.filtered_rows().len();
            format!("{scope}; {count} results; selected {}; query focused; Enter opens; Alt+Left back; Escape closes", if count == 0 { 0 } else { self.selected_index.min(count - 1) + 1 })
        })
    }

    pub fn move_selection_up(&mut self) {
        self.wheel_accumulated_y = 0.0;
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
                self.last_scroll_time = Some(Instant::now());
            }
        }
    }

    pub fn move_selection_down(&mut self) {
        self.wheel_accumulated_y = 0.0;
        let count = self.filtered_rows().len();
        if self.selected_index < count.saturating_sub(1) {
            self.selected_index += 1;
            if self.selected_index >= self.scroll_offset + self.visible_results {
                self.scroll_offset = self.selected_index - self.visible_results + 1;
                self.last_scroll_time = Some(Instant::now());
            }
        }
    }

    pub fn get_selected_action(&self) -> Option<PaletteAction> {
        self.filtered_rows()
            .get(self.selected_index)
            .and_then(|(_, row)| row.action())
    }

    /// Navigate without producing an executable action. Both pointer and key
    /// activation call this before inspecting the selected command.
    pub fn activate_navigation(&mut self) -> bool {
        let target =
            self.filtered_rows()
                .get(self.selected_index)
                .and_then(|(_, row)| {
                    if let PaletteRow::Navigation(category) = row {
                        Some(*category)
                    } else {
                        None
                    }
                });
        match target {
            Some(Some(category)) => {
                self.category = Some(category);
                self.set_query(String::new());
                self.selected_index = 1; // First command; Back remains one Up away.
                true
            }
            Some(None) => self.go_back(),
            None => false,
        }
    }

    pub fn go_back(&mut self) -> bool {
        let child_action = match self.mode {
            PaletteMode::Fonts(_) => Some(PaletteAction::ListFonts),
            PaletteMode::Market(_) => Some(PaletteAction::OpenMarket),
            _ => None,
        };
        if let Some(action) = child_action {
            self.mode = PaletteMode::Commands;
            self.category = Some(Category::for_action(action));
            self.set_query(String::new());
            self.selected_index = self
                .filtered_rows()
                .iter()
                .position(|(_, row)| row.action() == Some(action))
                .unwrap_or(0);
            return true;
        }
        if !matches!(
            self.mode,
            PaletteMode::Commands | PaletteMode::Fonts(_) | PaletteMode::Market(_)
        ) {
            return false;
        }
        if self.category.is_none() && matches!(self.mode, PaletteMode::Commands) {
            return false;
        }
        let parent = self.category.unwrap_or(Category::Tools);
        self.mode = PaletteMode::Commands;
        self.category = None;
        self.set_query(String::new());
        self.selected_index = Category::ALL
            .iter()
            .position(|category| *category == parent)
            .unwrap_or(0);
        true
    }

    fn can_go_back(&self) -> bool {
        self.enabled
            && match self.mode {
                PaletteMode::Commands => self.category.is_some(),
                PaletteMode::Fonts(_) | PaletteMode::Market(_) => true,
                _ => false,
            }
    }

    /// Fixed header geometry shared by drawing and pointer activation. The
    /// compact variant retains the arrow when a narrow viewport cannot fit text.
    fn back_button_rect(&self, dimensions: (f32, f32, f32)) -> Option<[f32; 4]> {
        if !self.can_go_back() {
            return None;
        }
        let (x, y, width, _, _) =
            self.palette_rect_for_count(dimensions.0, dimensions.1, dimensions.2, 1);
        Some([
            x + PALETTE_PADDING + 8.0,
            y + PALETTE_PADDING + 9.0,
            if width >= 360.0 { 76.0 } else { 28.0 },
            28.0,
        ])
    }

    pub fn try_back_click(
        &mut self,
        x: f32,
        y: f32,
        dimensions: (f32, f32, f32),
    ) -> bool {
        let Some([left, top, width, height]) = self.back_button_rect(dimensions) else {
            return false;
        };
        if x >= left && x < left + width && y >= top && y < top + height {
            return self.go_back();
        }
        false
    }

    /// Modal navigation is pure UI state: it cannot access a session or PTY.
    pub fn handle_navigation_key(
        &mut self,
        key: &rio_window::keyboard::Key,
        modifiers: rio_window::keyboard::ModifiersState,
        repeat: bool,
    ) -> bool {
        use rio_window::keyboard::{Key, ModifiersState, NamedKey};
        // A held Enter is consumed across category changes, never turned into
        // activation of the newly selected first command.
        if repeat && *key == Key::Named(NamedKey::Enter) {
            return true;
        }
        match key {
            Key::Named(NamedKey::Enter) if modifiers.is_empty() => {
                self.activate_navigation()
            }
            Key::Named(NamedKey::ArrowRight) if modifiers.is_empty() => {
                if matches!(
                    self.filtered_rows().get(self.selected_index),
                    Some((_, PaletteRow::Navigation(Some(_))))
                ) {
                    self.activate_navigation()
                } else {
                    false
                }
            }
            Key::Named(NamedKey::ArrowLeft) if modifiers == ModifiersState::ALT => {
                self.go_back()
            }
            Key::Named(NamedKey::Backspace)
                if modifiers.is_empty() && self.query.is_empty() =>
            {
                self.go_back()
            }
            Key::Named(NamedKey::Tab) if modifiers == ModifiersState::SHIFT => {
                self.move_selection_up();
                true
            }
            Key::Named(
                NamedKey::Home | NamedKey::End | NamedKey::PageUp | NamedKey::PageDown,
            ) if modifiers.is_empty() => {
                let total = self.filtered_rows().len();
                self.selected_index = match key {
                    Key::Named(NamedKey::Home) => 0,
                    Key::Named(NamedKey::End) => total.saturating_sub(1),
                    Key::Named(NamedKey::PageUp) => self
                        .selected_index
                        .saturating_sub(self.visible_results.max(1)),
                    _ => self
                        .selected_index
                        .saturating_add(self.visible_results.max(1))
                        .min(total.saturating_sub(1)),
                };
                self.wheel_accumulated_y = 0.0;
                self.scroll_offset = self.scroll_offset.min(self.selected_index);
                if self.selected_index >= self.scroll_offset + self.visible_results.max(1)
                {
                    self.scroll_offset =
                        self.selected_index + 1 - self.visible_results.max(1);
                }
                self.last_scroll_time = Some(Instant::now());
                true
            }
            _ => false,
        }
    }

    /// Selected family name if (and only if) the palette is in fonts
    /// mode and the selection points at a valid row. Owned `String`
    /// so the caller can mutate the palette state (`set_enabled`) in
    /// the same statement without fighting the borrow checker.
    pub fn get_selected_font(&self) -> Option<String> {
        self.filtered_rows()
            .get(self.selected_index)
            .and_then(|(_, row)| match row {
                PaletteRow::Font { family } => Some((*family).to_owned()),
                PaletteRow::Navigation(_) => None,
                PaletteRow::Command { .. }
                | PaletteRow::Market { .. }
                | PaletteRow::QuickAction { .. }
                | PaletteRow::QuickActionNotice { .. }
                | PaletteRow::PlaceholderContinue
                | PaletteRow::ReviewCommand { .. }
                | PaletteRow::ReviewInsert { .. }
                | PaletteRow::ReviewCopy { .. } => None,
            })
    }

    pub fn get_selected_market_id(&self) -> Option<String> {
        self.filtered_rows()
            .get(self.selected_index)
            .and_then(|(_, row)| match row {
                PaletteRow::Market { id, .. } => Some((*id).to_owned()),
                PaletteRow::Navigation(_) => None,
                PaletteRow::Command { .. }
                | PaletteRow::Font { .. }
                | PaletteRow::QuickAction { .. }
                | PaletteRow::QuickActionNotice { .. }
                | PaletteRow::PlaceholderContinue
                | PaletteRow::ReviewCommand { .. }
                | PaletteRow::ReviewInsert { .. }
                | PaletteRow::ReviewCopy { .. } => None,
            })
    }

    pub fn get_selected_action_item_id(&self) -> Option<String> {
        self.filtered_rows()
            .get(self.selected_index)
            .and_then(|(_, row)| match row {
                PaletteRow::QuickAction { item } => Some(item.id.clone()),
                _ => None,
            })
    }

    pub fn get_review_choice(&self) -> Option<QuickActionReviewChoice> {
        self.filtered_rows()
            .get(self.selected_index)
            .and_then(|(_, row)| match row {
                PaletteRow::ReviewInsert { .. } => Some(QuickActionReviewChoice::Insert),
                PaletteRow::ReviewCopy { .. } => Some(QuickActionReviewChoice::Copy),
                _ => None,
            })
    }

    /// Filtered list of rows for the current mode. Both modes share
    /// the same fuzzy-score + sort pipeline so typing behaves
    /// identically in either view.
    fn filtered_rows(&self) -> Vec<(i32, PaletteRow<'_>)> {
        let query = self.query.to_lowercase();
        let score = |target: &str| fuzzy_score_lowered(&query, target);
        let mut results: Vec<(i32, PaletteRow<'_>)> = match &self.mode {
            PaletteMode::Commands => {
                if self.query.is_empty() && self.category.is_none() {
                    return Category::ALL
                        .into_iter()
                        .map(|category| (0, PaletteRow::Navigation(Some(category))))
                        .collect();
                }
                let has_adaptive = self.has_adaptive_theme;
                COMMANDS
                    .iter()
                    .filter(|cmd| {
                        if self.query.is_empty()
                            && self.category.is_some_and(|category| {
                                Category::for_action(cmd.action) != category
                            })
                        {
                            return false;
                        }
                        if cmd.action == PaletteAction::ToggleAppearanceTheme {
                            return has_adaptive;
                        }
                        true
                    })
                    .filter_map(|cmd| {
                        let score = score(cmd.title).or_else(|| {
                            score(Category::for_action(cmd.action).title())
                                .map(|score| score - 100)
                        })?;
                        Some((
                            score,
                            PaletteRow::Command {
                                title: cmd.title,
                                shortcut: self.command_shortcut(cmd),
                                action: cmd.action,
                            },
                        ))
                    })
                    .collect()
            }
            PaletteMode::Fonts(fonts) => fonts
                .iter()
                .filter_map(|family| {
                    let score = score(family)?;
                    Some((score, PaletteRow::Font { family }))
                })
                .collect(),
            PaletteMode::Market(items) => items
                .iter()
                .filter_map(|item| {
                    let score = [
                        item.name.as_str(),
                        item.id.as_str(),
                        item.description.as_str(),
                    ]
                    .into_iter()
                    .filter_map(score)
                    .max()?;
                    Some((
                        score,
                        PaletteRow::Market {
                            id: &item.id,
                            name: &item.name,
                            installed: item.installed,
                        },
                    ))
                })
                .collect(),
            PaletteMode::QuickActions { items, notice } => {
                if items.is_empty() {
                    vec![(1, PaletteRow::QuickActionNotice { message: notice })]
                } else {
                    items
                        .iter()
                        .enumerate()
                        .map(|(index, item)| {
                            (
                                i32::try_from(items.len().saturating_sub(index))
                                    .unwrap_or(i32::MAX),
                                PaletteRow::QuickAction { item },
                            )
                        })
                        .collect()
                }
            }
            PaletteMode::QuickActionPlaceholder { .. } => {
                vec![(1, PaletteRow::PlaceholderContinue)]
            }
            PaletteMode::QuickActionReview(view) => {
                let mut rows = vec![(
                    3,
                    PaletteRow::ReviewCommand {
                        command: &view.command_preview,
                        context: &view.command_context_label,
                    },
                )];
                if view.copy_allowed {
                    if view.primary_label == "Copy command" {
                        rows.push((2, PaletteRow::ReviewCopy { risk: view.risk }));
                    } else {
                        rows.push((
                            2,
                            PaletteRow::ReviewInsert {
                                label: view.primary_label,
                                risk: view.risk,
                            },
                        ));
                        rows.push((1, PaletteRow::ReviewCopy { risk: view.risk }));
                    }
                }
                rows
            }
        };

        if !query.is_empty() {
            results.sort_by_key(|r| std::cmp::Reverse(r.0));
        }
        if self.category.is_some()
            && self.query.is_empty()
            && matches!(self.mode, PaletteMode::Commands)
        {
            results.insert(0, (i32::MAX, PaletteRow::Navigation(None)));
        }
        results
    }

    /// Returns the palette geometry for drawing and hit-testing. Width and row
    /// count contract before either edge can leave the live viewport.
    #[cfg(test)]
    fn palette_rect(
        &self,
        window_width: f32,
        window_height: f32,
        scale_factor: f32,
    ) -> (f32, f32, f32, f32, usize) {
        self.palette_rect_for_count(
            window_width,
            window_height,
            scale_factor,
            self.filtered_rows().len(),
        )
    }

    fn palette_rect_for_count(
        &self,
        window_width: f32,
        window_height: f32,
        scale_factor: f32,
        count: usize,
    ) -> (f32, f32, f32, f32, usize) {
        let viewport = Viewport::from_physical(window_width, window_height, scale_factor);
        let pw = viewport.fitted_surface(PALETTE_WIDTH, 8.0);
        let px = ((viewport.width - pw) / 2.0).max(0.0);
        let py = PALETTE_MARGIN_TOP.min((viewport.height * 0.12).max(8.0));
        let fixed_height =
            PALETTE_PADDING * 2.0 + INPUT_HEIGHT + SEPARATOR_HEIGHT + RESULTS_MARGIN_TOP;
        let available_height = (viewport.height - py - 8.0).max(0.0);
        let visible_results = (((available_height - fixed_height) / RESULT_ITEM_HEIGHT)
            .floor() as usize)
            .clamp(1, MAX_VISIBLE_RESULTS)
            .min(count.max(1));
        let h = PALETTE_PADDING
            + INPUT_HEIGHT
            + SEPARATOR_HEIGHT
            + RESULTS_MARGIN_TOP
            + RESULT_ITEM_HEIGHT * visible_results as f32
            + PALETTE_PADDING;
        (px, py, pw, h, visible_results)
    }

    /// Hit-test a mouse click. Returns Some(index) if a result row was clicked,
    /// or None if clicked outside the palette or on the input area.
    /// Returns Err(()) if clicked outside the palette entirely (should close).
    pub fn hit_test(
        &self,
        mouse_x: f32,
        mouse_y: f32,
        window_width: f32,
        window_height: f32,
        scale_factor: f32,
    ) -> Result<Option<usize>, ()> {
        let filtered_count = self.filtered_rows().len();
        let (px, py, pw, ph, visible_results) = self.palette_rect_for_count(
            window_width,
            window_height,
            scale_factor,
            filtered_count,
        );

        // Outside palette bounds
        if mouse_x < px || mouse_x > px + pw || mouse_y < py || mouse_y > py + ph {
            return Err(()); // Close palette
        }

        // Results area starts after input + separator
        let results_y =
            py + PALETTE_PADDING + INPUT_HEIGHT + SEPARATOR_HEIGHT + RESULTS_MARGIN_TOP;
        if mouse_y < results_y {
            return Ok(None); // Clicked on input area
        }

        let relative_y = mouse_y - results_y;
        let row = (relative_y / RESULT_ITEM_HEIGHT) as usize;
        if row >= visible_results {
            return Ok(None);
        }
        let actual_index =
            bounded_scroll_offset(filtered_count, visible_results, self.scroll_offset)
                + row;

        if actual_index < filtered_count {
            Ok(Some(actual_index))
        } else {
            Ok(None)
        }
    }

    /// Update selection based on mouse position. Returns true if selection changed.
    pub fn hover(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        window_width: f32,
        window_height: f32,
        scale_factor: f32,
    ) -> bool {
        if let Ok(Some(index)) =
            self.hit_test(mouse_x, mouse_y, window_width, window_height, scale_factor)
        {
            if self.selected_index != index {
                self.selected_index = index;
                return true;
            }
        }
        false
    }

    pub fn render(&mut self, sugarloaf: &mut Sugarloaf, dimensions: (f32, f32, f32)) {
        if !self.enabled {
            // Immediate mode: not drawing == not visible.
            return;
        }

        sugarloaf.begin_modal_layer();

        let (window_width, window_height, scale_factor) = dimensions;

        let filtered = self.filtered_rows();
        let (palette_x, palette_y, palette_width, palette_height, visible_results) = self
            .palette_rect_for_count(
                window_width,
                window_height,
                scale_factor,
                filtered.len(),
            );
        let scroll_offset = if self.selected_index < self.scroll_offset {
            self.selected_index
        } else if self.selected_index >= self.scroll_offset + visible_results {
            self.selected_index + 1 - visible_results
        } else {
            self.scroll_offset
        };

        sugarloaf.rect(
            None,
            0.0,
            0.0,
            window_width / scale_factor,
            window_height / scale_factor,
            BACKDROP_COLOR,
            DEPTH_BACKDROP,
            ORDER,
        );

        // Quiet card edges separate the overlay; cyan belongs to input/selection.
        sugarloaf.rounded_rect(
            None,
            palette_x - 8.0,
            palette_y + 5.0,
            palette_width + 16.0,
            palette_height + 12.0,
            SHADOW_COLOR,
            DEPTH_BG,
            PALETTE_CORNER_RADIUS + 7.0,
            ORDER,
        );
        stroke_rounded_rect(
            sugarloaf,
            palette_x,
            palette_y,
            palette_width,
            palette_height,
            1.0,
            PALETTE_CORNER_RADIUS,
            OUTLINE_COLOR,
            BG_COLOR,
            DEPTH_BG + 0.01,
            ORDER,
        );

        let input_x = palette_x + PALETTE_PADDING;
        let input_y = palette_y + PALETTE_PADDING;
        let input_width = palette_width - PALETTE_PADDING * 2.0;

        stroke_rounded_rect(
            sugarloaf,
            input_x,
            input_y,
            input_width,
            INPUT_HEIGHT - 4.0,
            1.0,
            CONTROL_RADIUS,
            INPUT_OUTLINE_COLOR,
            INPUT_BG_COLOR,
            DEPTH_ELEMENT,
            ORDER,
        );
        let input_icon_well =
            if let Some([x, y, width, height]) = self.back_button_rect(dimensions) {
                stroke_rounded_rect(
                    sugarloaf,
                    x,
                    y,
                    width,
                    height,
                    1.0,
                    KEYCAP_RADIUS,
                    OUTLINE_COLOR,
                    SHORTCUT_BG_COLOR,
                    DEPTH_ELEMENT + 0.02,
                    ORDER,
                );
                draw_command_icon(
                    sugarloaf,
                    CommandIcon::Back,
                    x + 4.0,
                    y + 4.0,
                    BRAND_CYAN,
                    SHORTCUT_BG_COLOR,
                );
                if width > 28.0 {
                    let opts = DrawOpts {
                        font_size: SHORTCUT_FONT_SIZE,
                        color: color_u8(TEXT_COLOR),
                        ..DrawOpts::default()
                    };
                    sugarloaf.text_mut().draw(x + 30.0, y + 7.0, "Back", &opts);
                }
                width + 8.0
            } else {
                draw_command_icon(
                    sugarloaf,
                    CommandIcon::Search,
                    input_x + 13.0,
                    input_y + 11.0,
                    BRAND_CYAN,
                    INPUT_BG_COLOR,
                );
                INPUT_ICON_WELL
            };

        let esc_x = input_x + input_width - ESC_BADGE_WIDTH - 10.0;
        stroke_rounded_rect(
            sugarloaf,
            esc_x,
            input_y + 11.0,
            ESC_BADGE_WIDTH,
            25.0,
            1.0,
            KEYCAP_RADIUS,
            OUTLINE_COLOR,
            SHORTCUT_BG_COLOR,
            DEPTH_ELEMENT + 0.01,
            ORDER,
        );
        let esc_opts = DrawOpts {
            font_size: SHORTCUT_FONT_SIZE,
            color: color_u8(SHORTCUT_TEXT_COLOR),
            ..DrawOpts::default()
        };
        sugarloaf
            .text_mut()
            .draw(esc_x + 9.0, input_y + 18.0, "ESC", &esc_opts);

        let placeholder = match self.mode {
            PaletteMode::Commands => self
                .category
                .map_or("Search all commands…", Category::title),
            PaletteMode::Fonts(_) => "Type a font name...",
            PaletteMode::Market(_) => "Search extensions...",
            PaletteMode::QuickActions { .. } => "Search Quick Actions...",
            PaletteMode::QuickActionPlaceholder { ref prompt } => prompt,
            PaletteMode::QuickActionReview(_) => "Review; command is never executed",
        };
        let input_text_width = (input_width
            - INPUT_PADDING_X * 2.0
            - input_icon_well
            - ESC_BADGE_WIDTH
            - 18.0)
            .max(0.0);
        let text_color = if self.query.is_empty() {
            DIM_TEXT_COLOR
        } else {
            TEXT_COLOR
        };

        let text_x = input_x + INPUT_PADDING_X + input_icon_well;
        let text_y = input_y + (INPUT_HEIGHT - INPUT_FONT_SIZE) / 2.0;
        let input_opts = DrawOpts {
            font_size: INPUT_FONT_SIZE,
            color: color_u8(text_color),
            ..DrawOpts::default()
        };
        let display_text = if self.query.is_empty() {
            elide_end(sugarloaf, placeholder, input_text_width, &input_opts)
        } else {
            elide_start(
                sugarloaf,
                self.query.as_str(),
                input_text_width,
                &input_opts,
            )
        };
        let input_rendered_width =
            sugarloaf
                .text_mut()
                .draw(text_x, text_y, &display_text, &input_opts);

        let elapsed_ms = self.caret_blink_start.elapsed().as_millis();
        let caret_visible = (elapsed_ms / CARET_BLINK_MS).is_multiple_of(2);

        if caret_visible {
            let text_width = if self.query.is_empty() {
                0.0
            } else {
                input_rendered_width
            };

            let caret_x = text_x + text_width;
            let caret_height = INPUT_FONT_SIZE + 4.0;
            let caret_y = input_y + (INPUT_HEIGHT - caret_height) / 2.0 + 2.0;

            sugarloaf.rect(
                None,
                caret_x,
                caret_y,
                CARET_WIDTH,
                caret_height,
                TEXT_COLOR,
                DEPTH_ELEMENT,
                ORDER,
            );
        }

        let sep_y = input_y + INPUT_HEIGHT;
        sugarloaf.rect(
            None,
            palette_x + PALETTE_PADDING,
            sep_y,
            palette_width - PALETTE_PADDING * 2.0,
            SEPARATOR_HEIGHT,
            SEPARATOR_COLOR,
            DEPTH_ELEMENT,
            ORDER,
        );

        let results_y = sep_y + SEPARATOR_HEIGHT + RESULTS_MARGIN_TOP;
        let effective_scroll_offset =
            bounded_scroll_offset(filtered.len(), visible_results, scroll_offset);

        for (display_i, (_, row)) in filtered
            .iter()
            .skip(effective_scroll_offset)
            .take(visible_results)
            .enumerate()
        {
            let actual_index = effective_scroll_offset + display_i;
            let item_y = results_y + RESULT_ITEM_HEIGHT * display_i as f32;
            let is_selected = actual_index == self.selected_index;
            let presentation = row.presentation();

            if is_selected {
                sugarloaf.rounded_rect(
                    None,
                    input_x,
                    item_y,
                    input_width,
                    RESULT_ITEM_HEIGHT - 2.0,
                    SELECTED_BG_COLOR,
                    DEPTH_ELEMENT,
                    CONTROL_RADIUS,
                    ORDER,
                );
                sugarloaf.rounded_rect(
                    None,
                    input_x + 4.0,
                    item_y + 11.0,
                    2.0,
                    RESULT_ITEM_HEIGHT - 24.0,
                    BRAND_CYAN,
                    DEPTH_ELEMENT + 0.02,
                    1.0,
                    ORDER,
                );
            }

            let icon_x = input_x + 14.0;
            let icon_y = item_y + (RESULT_ITEM_HEIGHT - RESULT_ICON_SIZE) / 2.0 - 1.0;
            let row_fill_color = if is_selected {
                SELECTED_BG_COLOR
            } else {
                BG_COLOR
            };
            draw_command_icon(
                sugarloaf,
                presentation.icon,
                icon_x,
                icon_y,
                presentation.accent,
                row_fill_color,
            );

            let result_opts = DrawOpts {
                font_size: RESULT_FONT_SIZE,
                color: color_u8(if is_selected {
                    TEXT_COLOR
                } else {
                    [0.68, 0.78, 0.86, 1.0]
                }),
                ..DrawOpts::default()
            };
            let row_text_x = icon_x + RESULT_ICON_SIZE + 14.0;
            let row_text_y = item_y + (RESULT_ITEM_HEIGHT - RESULT_FONT_SIZE) / 2.0 - 1.0;
            let shortcut = row.shortcut();
            let shortcut_opts = DrawOpts {
                font_size: SHORTCUT_FONT_SIZE,
                color: color_u8(if is_selected {
                    TEXT_COLOR
                } else {
                    SHORTCUT_TEXT_COLOR
                }),
                ..DrawOpts::default()
            };
            // Larger key labels must not crowd titles in narrow viewports.
            // Full shortcuts remain in the action/accessible model.
            let shortcut_display = elide_end(
                sugarloaf,
                shortcut,
                trailing_label_max_width(input_width),
                &shortcut_opts,
            );
            let is_font_row = matches!(row, PaletteRow::Font { .. });
            let trailing_width = if !shortcut.is_empty() {
                sugarloaf
                    .text_mut()
                    .measure(&shortcut_display, &shortcut_opts)
                    + 30.0
            } else if is_font_row {
                COPY_ICON_W + 24.0
            } else {
                10.0
            };
            let row_title = elide_end(
                sugarloaf,
                row.title(),
                (input_x + input_width - row_text_x - trailing_width).max(0.0),
                &result_opts,
            );
            sugarloaf
                .text_mut()
                .draw(row_text_x, row_text_y, &row_title, &result_opts);

            if !shortcut.is_empty() {
                let shortcut_width = sugarloaf
                    .text_mut()
                    .measure(&shortcut_display, &shortcut_opts);
                let keycap_width = shortcut_width + 18.0;
                let shortcut_x = input_x + input_width - 10.0 - keycap_width;
                let shortcut_y = item_y + 10.0;
                sugarloaf.rounded_rect(
                    None,
                    shortcut_x,
                    shortcut_y,
                    keycap_width,
                    24.0,
                    SHORTCUT_BG_COLOR,
                    DEPTH_ELEMENT + 0.01,
                    KEYCAP_RADIUS,
                    ORDER,
                );
                sugarloaf.text_mut().draw(
                    shortcut_x + 9.0,
                    shortcut_y + 6.0,
                    &shortcut_display,
                    &shortcut_opts,
                );
            }

            if is_font_row {
                let stroke_color = if is_selected {
                    TEXT_COLOR
                } else {
                    SHORTCUT_TEXT_COLOR
                };
                // Cutout inside each page uses the row's own background
                // so the border reads as a clean outline on either
                // palette-bg (idle) or selection-highlight-bg (hovered).
                let row_fill_color = if is_selected {
                    SELECTED_BG_COLOR
                } else {
                    BG_COLOR
                };
                let icon_x = input_x + input_width - 15.0 - COPY_ICON_W;
                let icon_y = item_y + (RESULT_ITEM_HEIGHT - COPY_ICON_H) / 2.0 - 1.0;
                draw_copy_icon(
                    sugarloaf,
                    icon_x,
                    icon_y,
                    stroke_color,
                    row_fill_color,
                    DEPTH_ELEMENT,
                    ORDER,
                );
            }
        }

        if filtered.is_empty() {
            let empty_opts = DrawOpts {
                font_size: RESULT_FONT_SIZE,
                color: color_u8(DIM_TEXT_COLOR),
                ..DrawOpts::default()
            };
            let empty = "No matching Automexia actions";
            let width = sugarloaf.text_mut().measure(empty, &empty_opts);
            sugarloaf.text_mut().draw(
                palette_x + (palette_width - width) / 2.0,
                results_y + 15.0,
                empty,
                &empty_opts,
            );
        }

        // Scrollbar: shares the terminal scrollbar's branded 6 px thumb and
        // fade envelope. Overflow always keeps a subdued indicator visible;
        // wheel, trackpad, or keyboard scrolling brightens it immediately.
        let total = filtered.len();
        self.visible_results = visible_results;
        if effective_scroll_offset != self.scroll_offset {
            self.wheel_accumulated_y = 0.0;
            self.scroll_offset = effective_scroll_offset;
        }
        let track_height = visible_results as f32 * RESULT_ITEM_HEIGHT;
        let normalized = if total > visible_results {
            effective_scroll_offset as f32 / (total - visible_results) as f32
        } else {
            0.0
        };
        if let Some((thumb_y, thumb_height)) = scrollbar::compute_thumb(
            visible_results,
            total,
            results_y,
            track_height,
            normalized,
        ) {
            let opacity = self.scrollbar_opacity();
            let bar_x = input_x + input_width
                - scrollbar::SCROLLBAR_WIDTH
                - scrollbar::SCROLLBAR_MARGIN;
            // Palette backdrop + bg rects use ORDER=20; the terminal
            // scrollbar's default ORDER=5 would land *under* them and
            // be invisible. Piggy-back on the palette's own order, at
            // a depth slightly above the selection highlight so a
            // hovered row doesn't mask the thumb.
            scrollbar::draw_thumb(
                sugarloaf,
                bar_x,
                thumb_y,
                thumb_height,
                opacity,
                false,
                DEPTH_ELEMENT + 0.05,
                ORDER,
            );
        }
        sugarloaf.end_modal_layer();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_enabled_resets_state() {
        let mut palette = CommandPalette::new();
        palette.set_query("test".to_string());
        palette.selected_index = 3;
        palette.scroll_offset = 2;

        palette.set_enabled(true);

        assert!(palette.query.is_empty());
        assert_eq!(palette.selected_index, 0);
        assert_eq!(palette.scroll_offset, 0);
    }

    #[test]
    fn test_filtered_commands_empty_query() {
        let mut palette = CommandPalette::new();
        palette.category = Some(Category::Appearance);
        let filtered = palette.filtered_rows();
        assert!(!filtered
            .iter()
            .any(|(_, row)| row.action() == Some(PaletteAction::ToggleAppearanceTheme)));
        palette.has_adaptive_theme = true;
        assert!(palette
            .filtered_rows()
            .iter()
            .any(|(_, row)| row.action() == Some(PaletteAction::ToggleAppearanceTheme)));
    }

    #[test]
    fn test_filtered_commands_by_title() {
        let mut palette = CommandPalette::new();
        palette.query = "split".to_string();
        let filtered = palette.filtered_rows();
        assert!(filtered.len() >= 2);
        for (_, row) in &filtered {
            assert!(row.title().to_lowercase().contains("split"));
        }
    }

    #[test]
    fn test_filtered_commands_case_insensitive() {
        let mut palette = CommandPalette::new();
        palette.query = "QUIT".to_string();
        let filtered = palette.filtered_rows();
        assert!(filtered.iter().any(|(_, row)| row.title() == "Quit"));
    }

    #[test]
    fn market_command_uses_plain_label() {
        let market = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::OpenMarket)
            .expect("market command");
        assert_eq!(market.title, "Extensions");
        assert!(!market.title.starts_with('/'));
    }

    #[test]
    fn every_command_has_a_vector_icon_and_opaque_accent() {
        for command in COMMANDS {
            let presentation = command_presentation(command.action);
            assert_eq!(presentation.accent[3], 1.0, "accent: {}", command.title);
            assert!(
                presentation.accent[..3]
                    .iter()
                    .all(|channel| channel.is_finite()),
                "non-finite icon accent: {}",
                command.title
            );
        }
    }

    #[test]
    fn primary_actions_have_distinct_icons_on_one_optical_grid() {
        assert_eq!(RESULT_ICON_SIZE, 22.0);
        let icons = [
            PaletteAction::TabCreate,
            PaletteAction::LocalTabCreate,
            PaletteAction::TabClose,
            PaletteAction::TabCloseUnfocused,
            PaletteAction::SelectNextTab,
            PaletteAction::SelectPrevTab,
            PaletteAction::SplitRight,
            PaletteAction::SplitDown,
            PaletteAction::CloneSplitRight,
            PaletteAction::CloneSplitDown,
            PaletteAction::SelectNextSplit,
            PaletteAction::SelectPrevSplit,
            PaletteAction::ConfigEditor,
            PaletteAction::WindowCreateNew,
            PaletteAction::ToggleFullscreen,
            PaletteAction::ToggleAppearanceTheme,
            PaletteAction::OpenConnections,
            PaletteAction::OpenMarket,
            PaletteAction::Quit,
        ]
        .map(|action| command_presentation(action).icon);

        for (index, icon) in icons.iter().enumerate() {
            assert!(
                !icons
                    .iter()
                    .skip(index + 1)
                    .any(|candidate| candidate == icon),
                "primary command icons must remain visually distinct: {icon:?}"
            );
        }
    }

    #[test]
    fn destructive_automexia_and_pane_commands_have_distinct_roles() {
        assert_eq!(
            command_presentation(PaletteAction::Quit).accent,
            BRAND_CORAL
        );
        assert_eq!(
            command_presentation(PaletteAction::OpenMarket).accent,
            BRAND_LIME
        );
        assert_eq!(
            command_presentation(PaletteAction::SplitRight).accent,
            BRAND_PURPLE
        );
        assert_ne!(
            command_presentation(PaletteAction::OpenMarket).icon,
            command_presentation(PaletteAction::SplitRight).icon
        );
    }

    #[test]
    fn test_fuzzy_matching() {
        let mut palette = CommandPalette::new();
        palette.query = "nt".to_string(); // Should match tab commands.
        let filtered = palette.filtered_rows();
        assert!(!filtered.is_empty());
    }

    #[test]
    fn opening_palette_browses_categories_without_executing_a_command() {
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        let titles: Vec<_> = palette
            .filtered_rows()
            .iter()
            .map(|(_, row)| row.title())
            .collect();
        assert_eq!(
            titles,
            [
                "Tabs & Windows",
                "Panes & Sessions",
                "Search & History",
                "Clipboard & Input",
                "Appearance",
                "Tools"
            ]
        );
        assert_eq!(palette.get_selected_action(), None);
    }

    #[test]
    fn test_set_query_resets_selection_and_scroll() {
        let mut palette = CommandPalette::new();
        palette.selected_index = 5;
        palette.scroll_offset = 3;
        palette.set_query("test".to_string());
        assert_eq!(palette.selected_index, 0);
        assert_eq!(palette.scroll_offset, 0);
    }

    #[test]
    fn test_move_selection_down() {
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        assert_eq!(palette.selected_index, 0);
        palette.move_selection_down();
        assert_eq!(palette.selected_index, 1);
        palette.move_selection_down();
        assert_eq!(palette.selected_index, 2);
    }

    #[test]
    fn test_move_selection_down_boundary() {
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        let count = palette.filtered_rows().len();
        palette.selected_index = count - 1;
        palette.move_selection_down();
        assert_eq!(palette.selected_index, count - 1);
    }

    #[test]
    fn test_move_selection_up() {
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        palette.selected_index = 3;
        palette.move_selection_up();
        assert_eq!(palette.selected_index, 2);
    }

    #[test]
    fn test_move_selection_up_boundary() {
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        palette.move_selection_up();
        assert_eq!(palette.selected_index, 0);
    }

    #[test]
    fn test_get_selected_action() {
        let mut palette = CommandPalette::new();
        assert!(palette.activate_navigation());
        let action = palette.get_selected_action();
        assert!(action.is_some());
        // First command is the current-window tab action.
        assert_eq!(action.unwrap(), PaletteAction::TabCreate);
    }

    #[test]
    fn window_window_tab_and_session_tab_are_distinct_commands() {
        let new_window = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::WindowCreateNew)
            .expect("new-window command should be present");
        let window_tab = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::TabCreate)
            .expect("window-tab command should be present");
        let local_tab = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::LocalTabCreate)
            .expect("session-tab command should be present");

        assert_eq!(new_window.title, "New Window");
        assert_eq!(window_tab.title, "New Window Tab");
        assert_eq!(local_tab.title, "New Tab in Selected Session");
        assert_ne!(new_window.action, window_tab.action);
        assert_ne!(window_tab.action, local_tab.action);
        assert_ne!(new_window.shortcut, window_tab.shortcut);
        assert_ne!(window_tab.shortcut, local_tab.shortcut);

        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(new_window.shortcut, "Ctrl+Shift+N");
            assert_eq!(window_tab.shortcut, "Ctrl+T");
            assert_eq!(local_tab.shortcut, "Ctrl+Shift+T");
        }
    }

    #[test]
    fn clone_and_default_split_commands_are_distinct_and_discoverable() {
        let clone_right = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::CloneSplitRight)
            .expect("clone-right command should be present");
        let clone_down = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::CloneSplitDown)
            .expect("clone-down command should be present");
        let split_right = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::SplitRight)
            .expect("fresh split-right command should be present");
        let split_down = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::SplitDown)
            .expect("fresh split-down command should be present");

        assert_eq!(clone_right.shortcut, SHORTCUT_CLONE_RIGHT);
        assert_eq!(clone_down.shortcut, SHORTCUT_CLONE_DOWN);
        assert_ne!(clone_right.action, split_right.action);
        assert_ne!(clone_down.action, split_down.action);
        assert_ne!(
            command_presentation(clone_right.action).icon,
            command_presentation(split_right.action).icon
        );
        assert_ne!(
            command_presentation(clone_down.action).icon,
            command_presentation(split_down.action).icon
        );
    }

    #[test]
    fn image_preview_command_is_discoverable() {
        let preview = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::PreviewSelectedImage)
            .expect("image preview command should be present");
        assert_eq!(preview.title, "Preview Selected Image");
        #[cfg(target_os = "macos")]
        assert_eq!(preview.shortcut, "Cmd+Alt+I");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(preview.shortcut, "Ctrl+Alt+I");
        assert_eq!(
            command_presentation(preview.action).icon,
            CommandIcon::Image
        );
    }

    #[test]
    fn command_jump_actions_are_directional_and_discoverable() {
        let command = |action| {
            COMMANDS
                .iter()
                .find(|command| command.action == action)
                .expect("command navigation action")
        };
        let previous = command(PaletteAction::ScrollToPreviousCommand);
        let next = command(PaletteAction::ScrollToNextCommand);

        assert_eq!(previous.title, "Jump to Previous Command");
        assert_eq!(next.title, "Jump to Next Command");
        #[cfg(target_os = "macos")]
        {
            assert_eq!(previous.shortcut, "Cmd+Shift+Up");
            assert_eq!(next.shortcut, "Cmd+Shift+Down");
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(previous.shortcut, "Ctrl+Shift+Up");
            assert_eq!(next.shortcut, "Ctrl+Shift+Down");
        }
        assert_ne!(
            command_presentation(previous.action).accent,
            command_presentation(next.action).accent
        );
    }
    #[test]
    fn strict_profile_palette_labels_come_from_the_compiled_registry() {
        let bindings = automexia_keybindings::bundled_profile(
            automexia_keybindings::ProfileId::Ghostty13,
            automexia_keybindings::PlatformFamily::LinuxBsd,
        )
        .unwrap();
        let registry = automexia_keybindings::compile(&bindings).registry.unwrap();
        let mut palette = CommandPalette::new();
        palette.set_binding_registry(
            Some(&registry),
            automexia_keybindings::ProfileId::Ghostty13,
            &[],
        );
        let shortcut = |action| {
            let command = COMMANDS
                .iter()
                .find(|command| command.action == action)
                .unwrap();
            palette.command_shortcut(command).to_string()
        };
        assert_eq!(shortcut(PaletteAction::TabCreate), "ctrl+shift+t");
        assert_eq!(shortcut(PaletteAction::SplitRight), "ctrl+shift+o");
        assert_eq!(shortcut(PaletteAction::LocalTabCreate), "Enter");
        assert_eq!(
            shortcut(PaletteAction::ScrollToPreviousCommand),
            "ctrl+shift+page_up"
        );
        assert_eq!(
            shortcut(PaletteAction::ScrollToNextCommand),
            "ctrl+shift+page_down"
        );
    }

    #[test]
    fn automexia_typed_unbind_offers_palette_enter() {
        let registry = automexia_keybindings::compile(&[]).registry.unwrap();
        let mut palette = CommandPalette::new();
        palette.set_binding_registry(
            Some(&registry),
            automexia_keybindings::ProfileId::Automexia,
            &["ctrl+t".into()],
        );
        let command = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::TabCreate)
            .unwrap();
        assert_eq!(palette.command_shortcut(command), "Enter");
    }

    #[test]
    fn palette_shortcuts_are_complete_and_unique() {
        let mut shortcuts = std::collections::HashMap::new();
        for command in COMMANDS {
            assert!(
                !command.shortcut.is_empty(),
                "missing palette shortcut for {}",
                command.title
            );
            assert!(
                shortcuts.insert(command.shortcut, command.title).is_none(),
                "duplicate palette shortcut {}",
                command.shortcut
            );
        }
    }

    #[test]
    fn clone_labels_follow_user_bindings_typed_overrides_and_reset() {
        use crate::bindings::{config_key_bindings, registry};
        use rio_backend::config::bindings::KeyBinding;
        let mut palette = CommandPalette::new();
        let command = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::CloneSplitRight)
            .unwrap();
        let bindings = config_key_bindings(
            vec![KeyBinding {
                key: "r".into(),
                action: "CloneSplitRight".into(),
                with: "control".into(),
                esc: String::new(),
                mode: String::new(),
            }],
            vec![],
        );
        for (typed, expected) in [
            (vec![], "ctrl+r"),
            (vec!["ctrl+r=unbind"], "Enter"),
            (vec!["ctrl+r=quit"], "Enter"),
            (vec!["ctrl+r>ctrl+x=quit"], "Enter"),
            (vec!["ctrl+d=unbind"], "ctrl+r"),
        ] {
            let mut config = rio_backend::config::Config::default();
            config.bindings.keybinds = typed.into_iter().map(str::to_string).collect();
            let snapshot = registry::build(&config).unwrap();
            palette.set_effective_bindings(&bindings, snapshot.as_ref());
            assert_eq!(palette.command_shortcut(command), expected);
        }
        palette.set_effective_bindings(&[], None);
        assert_eq!(palette.command_shortcut(command), "Enter");
    }

    #[test]
    fn pane_and_local_tab_navigation_are_scoped_and_discoverable() {
        let command = |action| {
            COMMANDS
                .iter()
                .find(|command| command.action == action)
                .expect("navigation command")
        };
        assert_eq!(
            command(PaletteAction::SelectNextLocalTab).title,
            "Next Tab in Selected Pane"
        );
        assert_eq!(
            command(PaletteAction::SelectPrevLocalTab).title,
            "Previous Tab in Selected Pane"
        );
        assert!(!command(PaletteAction::SelectPaneLeft).shortcut.is_empty());
        assert!(!command(PaletteAction::SelectPaneRight).shortcut.is_empty());
        assert!(!command(PaletteAction::SelectPaneUp).shortcut.is_empty());
        assert!(!command(PaletteAction::SelectPaneDown).shortcut.is_empty());
        assert_eq!(
            command(PaletteAction::SelectNextSplit).shortcut,
            SHORTCUT_NEXT_PANE
        );
        assert_eq!(
            command(PaletteAction::SelectPrevSplit).shortcut,
            SHORTCUT_PREV_PANE
        );
    }

    #[test]
    fn test_get_selected_action_with_filter() {
        let mut palette = CommandPalette::new();
        palette.set_query("quit".to_string());
        let action = palette.get_selected_action();
        assert_eq!(action, Some(PaletteAction::Quit));
    }

    #[test]
    fn test_scroll_offset_on_move_down() {
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        palette.selected_index = 1;
        assert!(palette.activate_navigation());
        for _ in 0..MAX_VISIBLE_RESULTS {
            palette.move_selection_down();
        }
        assert!(palette.scroll_offset > 0);
    }

    #[test]
    fn test_hit_test_outside() {
        let palette = CommandPalette::new();
        assert!(palette.hit_test(0.0, 0.0, 1200.0, 760.0, 1.0).is_err());
    }

    #[test]
    fn palette_contracts_to_minimum_window() {
        let palette = CommandPalette::new();
        let (x, y, width, height, rows) = palette.palette_rect(300.0, 200.0, 1.0);
        assert!(x >= 0.0 && y >= 0.0);
        assert!(x + width <= 300.0);
        assert!(y + height <= 200.0);
        assert!((1..MAX_VISIBLE_RESULTS).contains(&rows));
    }

    #[test]
    fn palette_geometry_uses_logical_hidpi_size() {
        let palette = CommandPalette::new();
        let logical = palette.palette_rect(600.0, 400.0, 1.0);
        let hidpi = palette.palette_rect(1_200.0, 800.0, 2.0);
        assert_eq!(logical, hidpi);
    }

    #[test]
    fn headerless_search_surface_is_not_a_result_hit_target() {
        let palette = CommandPalette::new();
        let (x, y, width, height, rows) = palette.palette_rect(1_280.0, 760.0, 1.0);
        assert_eq!(
            height,
            PALETTE_PADDING
                + INPUT_HEIGHT
                + SEPARATOR_HEIGHT
                + RESULTS_MARGIN_TOP
                + RESULT_ITEM_HEIGHT * rows as f32
                + PALETTE_PADDING
        );
        assert_eq!(
            palette.hit_test(x + width / 2.0, y + 20.0, 1_280.0, 760.0, 1.0),
            Ok(None)
        );
        assert_eq!(
            palette.hit_test(x + 40.0, y + PALETTE_PADDING + 20.0, 1_280.0, 760.0, 1.0),
            Ok(None)
        );
    }

    #[test]
    fn test_fuzzy_score_basic() {
        assert!(fuzzy_score("nt", "New Global Tab").is_some());
        assert!(fuzzy_score("xyz", "New Global Tab").is_none());
        assert!(fuzzy_score("", "New Global Tab").is_some());
    }

    #[test]
    fn test_fuzzy_score_ordering() {
        // The creation command and navigation command should both remain searchable.
        let score_new = fuzzy_score("net", "New Global Tab").unwrap_or(-100);
        let score_next = fuzzy_score("net", "Next Tab").unwrap_or(-100);
        // Both should match
        assert!(score_new > -100);
        assert!(score_next > -100);
    }

    #[test]
    fn enter_fonts_mode_switches_to_font_list() {
        let mut palette = CommandPalette::new();
        palette.set_enabled(true);
        palette.set_query("ab".to_string());
        palette.selected_index = 2;

        let fonts = vec![
            "JetBrains Mono".to_string(),
            "Fira Code".to_string(),
            "Cascadia Code".to_string(),
        ];
        palette.enter_fonts_mode(fonts);

        // Query cleared, selection reset, full list visible.
        assert!(palette.query.is_empty());
        assert_eq!(palette.selected_index, 0);
        assert_eq!(palette.filtered_rows().len(), 3);
        // Every row is a Font row, so no executable action.
        assert!(palette.get_selected_action().is_none());
    }

    #[test]
    fn fonts_mode_filters_by_fuzzy_score() {
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode(vec![
            "JetBrains Mono".to_string(),
            "Fira Code".to_string(),
            "Cascadia Code".to_string(),
        ]);
        palette.set_query("cas".to_string());
        let filtered = palette.filtered_rows();
        assert!(filtered.iter().any(|(_, r)| r.title() == "Cascadia Code"));
        assert!(filtered.iter().all(|(_, r)| {
            r.title().to_lowercase().contains('c')
                && r.title().to_lowercase().contains('a')
                && r.title().to_lowercase().contains('s')
        }));
    }

    #[test]
    fn fonts_mode_row_has_no_shortcut_column() {
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode(vec!["Fira Code".to_string()]);
        let filtered = palette.filtered_rows();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.shortcut(), "");
    }

    #[test]
    fn set_enabled_resets_fonts_mode_to_commands() {
        // Re-opening the palette with the keyboard must drop any stale
        // font list — reopening otherwise would land the user on fonts
        // they saw yesterday, which is surprising.
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode(vec!["Fira Code".to_string()]);
        palette.enabled = true;
        palette.set_enabled(false);
        palette.set_enabled(true);
        assert!(matches!(palette.mode, PaletteMode::Commands));
        // Commands list is back (non-empty modulo adaptive-theme filter).
        assert!(!palette.filtered_rows().is_empty());
    }

    #[test]
    fn get_selected_font_returns_family_in_fonts_mode() {
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode(vec![
            "JetBrains Mono".to_string(),
            "Fira Code".to_string(),
        ]);
        // First row (sorted alphabetically by fuzzy_score tie-break:
        // both score 0 with empty query, so first-inserted wins).
        let selected = palette.get_selected_font();
        assert!(selected.is_some());
        // The returned name must be one of the inputs, irrespective
        // of fuzzy-sort ordering.
        let s = selected.unwrap();
        assert!(s == "JetBrains Mono" || s == "Fira Code");
    }

    #[test]
    fn get_selected_font_none_in_commands_mode() {
        let palette = CommandPalette::new();
        // Default mode is Commands; no font to copy.
        assert!(palette.get_selected_font().is_none());
    }

    #[test]
    fn get_selected_font_none_when_empty_filter() {
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode(vec!["Fira Code".to_string()]);
        palette.set_query("zzzz".to_string());
        // Query doesn't match anything → no selected font.
        assert!(palette.get_selected_font().is_none());
    }

    // Scrollbar geometry + fade math live in `renderer::scrollbar` and
    // are tested there. The tests below cover the palette's own contract:
    // overflowing lists expose an idle indicator, scrolling brightens it,
    // and gesture state resets when the list reshapes.

    #[test]
    fn scrollbar_activity_starts_idle_until_first_scroll() {
        // Long list, palette just opened — no scroll activity has happened,
        // so the persistent overflow indicator remains at idle opacity.
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        assert!(palette.last_scroll_time.is_none());
        assert_eq!(palette.scrollbar_opacity(), PALETTE_SCROLLBAR_IDLE_OPACITY);
    }

    #[test]
    fn mouse_wheel_scrolls_long_palette_both_directions_and_keeps_selection_visible() {
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        palette.visible_results = 6;

        assert!(palette.scroll_line_delta(-3.0));
        assert_eq!(palette.scroll_offset, 3);
        assert!((3..9).contains(&palette.selected_index));
        assert!(palette.last_scroll_time.is_some());

        assert!(palette.scroll_line_delta(2.0));
        assert_eq!(palette.scroll_offset, 1);
        assert!((1..7).contains(&palette.selected_index));
    }

    #[test]
    fn trackpad_pixels_accumulate_without_losing_direction_or_overscrolling() {
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        palette.visible_results = 5;

        assert!(!palette.scroll_pixel_delta(-20.0));
        assert_eq!(palette.scroll_offset, 0);
        assert!(palette.scroll_pixel_delta(-25.0));
        assert_eq!(palette.scroll_offset, 1);

        // Reversing direction starts a fresh gesture instead of making the
        // user cancel a stale fractional delta first.
        assert!(!palette.scroll_pixel_delta(22.0));
        assert!(palette.scroll_pixel_delta(22.0));
        assert_eq!(palette.scroll_offset, 0);

        // Boundary motion is consumed but cannot leave latent momentum that
        // delays the next gesture in the opposite direction.
        assert!(!palette.scroll_line_delta(100.0));
        assert!(palette.scroll_line_delta(-100.0));
        assert_eq!(palette.scroll_offset, 45);
        assert_eq!(palette.selected_index, 45);
    }

    #[test]
    fn resize_and_keyboard_transitions_drop_stale_fractional_motion() {
        assert_eq!(bounded_scroll_offset(50, 5, 45), 45);
        assert_eq!(bounded_scroll_offset(50, 10, 45), 40);
        assert_eq!(bounded_scroll_offset(4, 10, 3), 0);

        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        palette.visible_results = 5;
        assert!(!palette.scroll_pixel_delta(-20.0));
        palette.move_selection_down();
        assert_eq!(palette.wheel_accumulated_y, 0.0);
        assert!(!palette.scroll_pixel_delta(-24.0));
        assert_eq!(palette.scroll_offset, 0);
    }

    #[test]
    fn wheel_is_bounded_for_short_empty_and_reshaped_lists() {
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode(vec!["Fira Code".into(), "JetBrains Mono".into()]);
        palette.visible_results = 10;
        assert!(!palette.scroll_line_delta(-3.0));
        assert_eq!(palette.scroll_offset, 0);

        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        palette.visible_results = 4;
        assert!(palette.scroll_line_delta(-8.0));
        palette.set_query("Family 00".into());
        assert_eq!(palette.scroll_offset, 0);
        assert!(!palette.scroll_line_delta(-1.0));

        palette.enter_fonts_mode(Vec::new());
        assert!(!palette.scroll_line_delta(-1.0));
        assert_eq!(palette.selected_index, 0);

        assert!(!palette.scroll_line_delta(f32::INFINITY));
        assert!(!palette.scroll_pixel_delta(f64::NAN));

        palette.enter_fonts_mode((0..3_000).map(|i| format!("Family {i:04}")).collect());
        palette.visible_results = 4;
        assert!(palette.scroll_line_delta(-f32::MAX));
        assert_eq!(palette.scroll_offset, MAX_WHEEL_ROWS_PER_EVENT as usize);
        assert!(palette.scroll_pixel_delta(-f64::MAX));
        assert_eq!(palette.scroll_offset, 2 * MAX_WHEEL_ROWS_PER_EVENT as usize);
    }

    #[test]
    fn overflowing_palette_exposes_an_idle_indicator_and_brightens_on_scroll() {
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        palette.visible_results = 6;

        assert_eq!(palette.scrollbar_opacity(), PALETTE_SCROLLBAR_IDLE_OPACITY);
        assert!(palette.scroll_line_delta(-1.0));
        assert_eq!(palette.scrollbar_opacity(), 1.0);
    }

    #[test]
    fn scrollbar_triggered_only_when_offset_actually_changes() {
        // The first few `move_selection_down` calls don't change
        // `scroll_offset` (selection walks within the visible window).
        // Only when selection crosses the window boundary does
        // `scroll_offset` bump, and only then does the scrollbar wake.
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        for _ in 0..MAX_VISIBLE_RESULTS {
            palette.move_selection_down();
        }
        // At this point selection has just crossed into scroll territory.
        assert!(palette.last_scroll_time.is_some());
    }

    #[test]
    fn scrollbar_timer_reset_on_query_change() {
        // Typing re-filters the list, which can shrink it below the
        // visible window. Any stale scrollbar timer must clear so a
        // leftover thumb doesn't linger over the new short list.
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        for _ in 0..MAX_VISIBLE_RESULTS {
            palette.move_selection_down();
        }
        assert!(palette.last_scroll_time.is_some());
        palette.set_query("Family 00".to_string());
        assert!(palette.last_scroll_time.is_none());
    }

    #[test]
    fn scrollbar_timer_reset_on_palette_reopen() {
        // Closing and re-opening the palette must drop any lingering
        // scrollbar state so the user doesn't see a fading thumb on a
        // fresh palette.
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        for _ in 0..MAX_VISIBLE_RESULTS {
            palette.move_selection_down();
        }
        palette.set_enabled(false);
        palette.set_enabled(true);
        assert!(palette.last_scroll_time.is_none());
    }

    #[test]
    fn list_fonts_command_is_present_and_actionable() {
        // Confirms `List Fonts` shows up in the command list and
        // reports the correct action when selected.
        let mut palette = CommandPalette::new();
        palette.set_query("list fonts".to_string());
        let filtered = palette.filtered_rows();
        assert!(!filtered.is_empty());
        assert_eq!(filtered[0].1.title(), "List Fonts");
        palette.selected_index = 0;
        assert_eq!(
            palette.get_selected_action(),
            Some(PaletteAction::ListFonts)
        );
    }

    #[test]
    fn quick_action_results_keep_worker_order_and_stable_ids() {
        let mut palette = CommandPalette::new();
        palette.enter_action_search(
            vec![
                QuickActionListItem::new(
                    "git.status".into(),
                    "Git status".into(),
                    "Inspect worktree".into(),
                    "Session".into(),
                    QuickActionRisk::ReadOnly,
                    0,
                ),
                QuickActionListItem::new(
                    "cluster.delete".into(),
                    "Delete pod".into(),
                    "Destructive operation".into(),
                    "User".into(),
                    QuickActionRisk::Destructive,
                    1,
                ),
            ],
            "git".into(),
        );
        assert_eq!(
            palette.get_selected_action_item_id().as_deref(),
            Some("git.status")
        );
        assert_eq!(
            palette.filtered_rows()[0].1.shortcut(),
            "Read-only · Session"
        );
        palette.move_selection_down();
        assert_eq!(
            palette.get_selected_action_item_id().as_deref(),
            Some("cluster.delete")
        );
    }

    #[test]
    fn provider_actions_use_connection_visuals_and_show_context_in_review() {
        let mut ordinary = CommandPalette::new();
        ordinary.enter_action_review(QuickActionReviewView::new(
            "ordinary.action".into(),
            "Ordinary action".into(),
            "git status".into(),
            QuickActionRisk::ReadOnly,
            automexia_ui_model::quick_actions::QuickActionMode::Insert,
        ));
        assert_eq!(ordinary.filtered_rows()[0].1.shortcut(), "Exact command");

        let item = QuickActionListItem::new(
            "provider.aws.identity".into(),
            "Show AWS identity".into(),
            "Inspect identity".into(),
            "Environment capsule".into(),
            QuickActionRisk::ReadOnly,
            0,
        )
        .with_provider_context(
            "AWS · Account 123456789012 · Current · Production".into(),
        );
        let row = PaletteRow::QuickAction { item: &item };
        assert_eq!(row.presentation().icon, CommandIcon::Connections);
        assert_eq!(row.presentation().accent, BRAND_CYAN);
        assert_eq!(row.shortcut(), item.metadata_label);

        let mut palette = CommandPalette::new();
        palette.enter_action_review(
            QuickActionReviewView::new(
                item.id,
                item.name,
                "aws 'sts' 'get-caller-identity'".into(),
                QuickActionRisk::ReadOnly,
                automexia_ui_model::quick_actions::QuickActionMode::Insert,
            )
            .with_provider_context(item.metadata_label.clone(), true),
        );
        assert_eq!(palette.filtered_rows()[0].1.shortcut(), item.metadata_label);
    }

    #[test]
    fn quick_action_loading_and_empty_notices_are_visible_but_not_actionable() {
        let mut palette = CommandPalette::new();
        palette.enter_action_search(Vec::new(), String::new());
        assert_eq!(
            palette.filtered_rows()[0].1.title(),
            "Loading Quick Actions…"
        );
        assert!(palette.get_selected_action_item_id().is_none());
        assert!(palette.get_selected_action().is_none());

        palette.update_action_items(Vec::new(), "No matching Quick Actions".into());
        assert_eq!(
            palette.filtered_rows()[0].1.title(),
            "No matching Quick Actions"
        );
        assert!(palette.get_selected_action_item_id().is_none());
    }

    #[test]
    fn trailing_label_budget_is_proportional_and_bounded() {
        assert_eq!(trailing_label_max_width(100.0), 42.0);
        assert!((trailing_label_max_width(300.0) - 126.0).abs() < 0.001);
        assert_eq!(trailing_label_max_width(1_000.0), 220.0);
        for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(trailing_label_max_width(invalid), 0.0);
        }
    }

    #[test]
    fn readable_key_labels_keep_action_space_at_fractional_scales() {
        use rio_backend::sugarloaf::{
            font::{constants, FontData, FontLibrary, FontLibraryData},
            text::Text,
        };
        use std::sync::Arc;
        let mut data = FontLibraryData::default();
        data.insert(
            FontData::from_static_slice(constants::FONT_CASCADIA_CODE_NF).unwrap(),
        );
        let fonts = FontLibrary {
            inner: Arc::new(parking_lot::RwLock::new(data)),
        };
        let mut text = Text::new(&fonts);
        text.init_cpu();
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0, 4.0] {
            text.set_scale_factor(scale);
            let options = DrawOpts {
                font_size: SHORTCUT_FONT_SIZE,
                ..Default::default()
            };
            for width in [200.0, 280.0, 576.0, 7680.0] {
                for label in [
                    "Enter",
                    "Alt+Shift+R",
                    "Ctrl+Shift+PageDown",
                    "Read-only · Example context",
                    "⌘⇧ → e\u{301}",
                ] {
                    let before = label.to_owned();
                    let budget = trailing_label_max_width(width);
                    let fitted = crate::renderer::text_fit::fit_end(
                        label,
                        budget,
                        "…",
                        |candidate, _| text.measure(candidate, &options),
                    );
                    let advance = text.measure(&fitted.display, &options);
                    assert!(advance <= budget);
                    assert!(
                        advance + 30.0 + 50.0 < width,
                        "retain room for the action label"
                    );
                    assert_eq!(
                        label, before,
                        "display fitting cannot change accessible shortcut text"
                    );
                }
            }
        }
    }

    #[test]
    fn review_requires_an_explicit_operation_and_keeps_exact_command_visible() {
        let mut palette = CommandPalette::new();
        palette.enter_action_review(QuickActionReviewView::new(
            "git.status".into(),
            "Git status".into(),
            "git 'status'".into(),
            QuickActionRisk::ReadOnly,
            automexia_ui_model::quick_actions::QuickActionMode::Insert,
        ));
        assert_eq!(
            palette.get_review_choice(),
            Some(QuickActionReviewChoice::Insert)
        );
        palette.move_selection_up();
        assert!(palette.get_review_choice().is_none());
        assert_eq!(palette.filtered_rows()[0].1.title(), "git 'status'");
    }

    #[test]
    fn unavailable_review_has_no_insert_or_copy_choice() {
        let mut palette = CommandPalette::new();
        palette.enter_action_review(QuickActionReviewView::new(
            "blocked".into(),
            "Blocked".into(),
            "exact launch remains disabled".into(),
            QuickActionRisk::Privileged,
            automexia_ui_model::quick_actions::QuickActionMode::Unavailable,
        ));
        assert_eq!(palette.filtered_rows().len(), 1);
        assert!(palette.get_review_choice().is_none());
    }

    #[test]
    fn palette_query_is_bounded_and_rejects_controls() {
        let mut palette = CommandPalette::new();
        palette.set_query("safe".into());
        palette.set_query("x".repeat(MAX_PALETTE_QUERY_BYTES + 1));
        assert_eq!(palette.query, "safe");
        palette.set_query("unsafe\nquery".into());
        assert_eq!(palette.query, "safe");
    }
}
