// Copyright (c) 2023-present, Raphael Amorim.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use crate::automexia::marketplace::MarketItem;
use crate::renderer::responsive::{elide_end, elide_start, Viewport};
use crate::renderer::scrollbar;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;
use std::time::Instant;

/// Convert `[f32; 4]` colour to `[u8; 4]` for the `Text` API (the
/// vertex shader premultiplies, so pass non-premul RGBA).
#[inline]
fn color_u8(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
        (c[3].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

// Headerless command palette: search is the visual anchor and every action
// shares one crisp, DPI-independent icon grid.
const PALETTE_WIDTH: f32 = 600.0;
const PALETTE_CORNER_RADIUS: f32 = 16.0;
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
const SHORTCUT_FONT_SIZE: f32 = 10.0;
const MAX_VISIBLE_RESULTS: usize = 10;

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

// Colors — dark minimalist
const BACKDROP_COLOR: [f32; 4] = [0.0, 0.025, 0.055, 0.72];
const SHADOW_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.42];
const OUTLINE_COLOR: [f32; 4] = [0.055, 0.36, 0.58, 0.88];
const BG_COLOR: [f32; 4] = [0.008, 0.027, 0.050, 1.0];
const INPUT_BG_COLOR: [f32; 4] = [0.012, 0.046, 0.080, 1.0];
const INPUT_OUTLINE_COLOR: [f32; 4] = [0.075, 0.40, 0.61, 0.92];
const SELECTED_BG_COLOR: [f32; 4] = [0.022, 0.125, 0.205, 0.88];
const SELECTED_OUTLINE_COLOR: [f32; 4] = [0.063, 0.72, 0.96, 0.82];
const TEXT_COLOR: [f32; 4] = [0.86, 0.93, 0.98, 1.0];
const DIM_TEXT_COLOR: [f32; 4] = [0.38, 0.49, 0.59, 1.0];
const SHORTCUT_TEXT_COLOR: [f32; 4] = [0.57, 0.69, 0.78, 1.0];
const SHORTCUT_BG_COLOR: [f32; 4] = [0.020, 0.065, 0.105, 1.0];
const SHORTCUT_OUTLINE_COLOR: [f32; 4] = [0.080, 0.24, 0.35, 0.94];
const SEPARATOR_COLOR: [f32; 4] = [0.055, 0.19, 0.29, 0.84];
const BRAND_CYAN: [f32; 4] = [0.063, 0.88, 1.0, 1.0];
const BRAND_BLUE: [f32; 4] = [0.18, 0.58, 0.96, 1.0];
const BRAND_PURPLE: [f32; 4] = [0.78, 0.42, 1.0, 1.0];
const BRAND_LIME: [f32; 4] = [0.52, 0.94, 0.36, 1.0];
const BRAND_AMBER: [f32; 4] = [1.0, 0.69, 0.18, 1.0];
const BRAND_CORAL: [f32; 4] = [1.0, 0.36, 0.48, 1.0];

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
const SHORTCUT_CLOSE_TAB: &str = "Cmd+W";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CLOSE_TAB: &str = "Ctrl+Shift+W";
#[cfg(target_os = "macos")]
const SHORTCUT_CLOSE_SURFACE: &str = "Cmd+W";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CLOSE_SURFACE: &str = "";
#[cfg(target_os = "macos")]
const SHORTCUT_SPLIT_RIGHT: &str = "Cmd+D";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SPLIT_RIGHT: &str = "Ctrl+Shift+R";
#[cfg(target_os = "macos")]
const SHORTCUT_SPLIT_DOWN: &str = "Cmd+Shift+D";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SPLIT_DOWN: &str = "Ctrl+Shift+D";
const SHORTCUT_CLONE_RIGHT: &str = "Ctrl+R";
const SHORTCUT_CLONE_DOWN: &str = "Ctrl+D";
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
const SHORTCUT_SEARCH: &str = "Cmd+F";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_SEARCH: &str = "Ctrl+Shift+F";
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
const SHORTCUT_APPEARANCE: &str = "";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_APPEARANCE: &str = "Alt+Shift+T";
#[cfg(target_os = "macos")]
const SHORTCUT_PREVIEW_IMAGE: &str = "Cmd+Alt+I";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_PREVIEW_IMAGE: &str = "Ctrl+Alt+I";
#[cfg(target_os = "macos")]
const SHORTCUT_CLEAR_SCREEN: &str = "Cmd+K";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_CLEAR_SCREEN: &str = "Ctrl+Shift+K";
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
    SplitRight,
    SplitDown,
    CloneSplitRight,
    CloneSplitDown,
    SelectNextSplit,
    SelectPrevSplit,
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
    SearchForward,
    SearchBackward,
    PreviewSelectedImage,
    ClearScreen,
    CloseCurrentSplitOrTab,
    OpenMarket,
    /// Browse the family names of every registered font. Does NOT
    /// execute a one-shot action — the palette stays open with the
    /// font list as its contents. Handled by `router`, not
    /// `Screen::execute_palette_action`.
    ListFonts,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandIcon {
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
        SearchForward | SearchBackward => RowPresentation {
            icon: CommandIcon::Search,
            accent: BRAND_BLUE,
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
        shortcut: "",
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
        shortcut: "",
        action: PaletteAction::SelectNextSplit,
    },
    Command {
        title: "Previous Split",
        shortcut: "",
        action: PaletteAction::SelectPrevSplit,
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
        title: "Search Forward",
        shortcut: SHORTCUT_SEARCH,
        action: PaletteAction::SearchForward,
    },
    Command {
        title: "Search Backward",
        shortcut: "",
        action: PaletteAction::SearchBackward,
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
        title: "market",
        shortcut: "",
        action: PaletteAction::OpenMarket,
    },
    Command {
        title: "List Fonts",
        shortcut: "",
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
}

/// One row in the filtered result list. Variants carry exactly the
/// data the render pass needs — no `&'static Command` vs `&str`
/// lifetime mixing.
enum PaletteRow<'a> {
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
}

impl<'a> PaletteRow<'a> {
    fn title(&self) -> &'a str {
        match *self {
            PaletteRow::Command { title, .. } => title,
            PaletteRow::Font { family } => family,
            PaletteRow::Market { name, .. } => name,
        }
    }

    fn shortcut(&self) -> &'a str {
        match *self {
            PaletteRow::Command { shortcut, .. } => shortcut,
            PaletteRow::Font { .. } => "",
            PaletteRow::Market {
                installed: true, ..
            } => "Remove",
            PaletteRow::Market {
                installed: false, ..
            } => "Install",
        }
    }

    fn action(&self) -> Option<PaletteAction> {
        match *self {
            PaletteRow::Command { action, .. } => Some(action),
            PaletteRow::Font { .. } | PaletteRow::Market { .. } => None,
        }
    }

    fn presentation(&self) -> RowPresentation {
        match *self {
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
        }
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
fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();

    if query_lower.is_empty() {
        return Some(0);
    }

    let mut qi = 0;
    let mut score: i32 = 0;
    let mut prev_match = false;
    let mut first_match_pos = None;

    for (ti, &tc) in target_lower.iter().enumerate() {
        if qi < query_lower.len() && tc == query_lower[qi] {
            if first_match_pos.is_none() {
                first_match_pos = Some(ti);
            }
            // Consecutive match bonus
            if prev_match {
                score += 5;
            }
            // Word boundary bonus (start of string or after space/punctuation)
            if ti == 0 || !target_lower[ti - 1].is_alphanumeric() {
                score += 10;
            }
            prev_match = true;
            qi += 1;
        } else {
            prev_match = false;
        }
    }

    if qi < query_lower.len() {
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
    /// Which list the palette is showing (commands or fonts).
    mode: PaletteMode,
    /// Timestamp for caret blinking
    caret_blink_start: Instant,
    /// Timestamp of the last event that actually changed `scroll_offset`.
    /// Drives the scrollbar fade-in/fade-out, sharing the terminal
    /// scrollbar's 2 s delay + 300 ms fade envelope via
    /// `scrollbar::opacity_from_last_scroll`. `None` while the palette
    /// has never scrolled since it opened — scrollbar stays hidden.
    last_scroll_time: Option<Instant>,
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
            mode: PaletteMode::Commands,
            caret_blink_start: Instant::now(),
            last_scroll_time: None,
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
            // Always re-open into Commands mode — a stale Fonts list
            // from a previous session would be misleading (fonts may
            // have changed) and surprising (user toggles palette and
            // finds themselves on the font list).
            self.mode = PaletteMode::Commands;
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
    }

    pub fn enter_market_mode(&mut self, items: Vec<MarketItem>) {
        self.mode = PaletteMode::Market(items);
        self.query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.caret_blink_start = Instant::now();
        self.last_scroll_time = None;
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.caret_blink_start = Instant::now();
        // Typing reshapes the list entirely — drop any scrollbar
        // fade state so the next scroll starts with a clean timer.
        self.last_scroll_time = None;
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
                self.last_scroll_time = Some(Instant::now());
            }
        }
    }

    pub fn move_selection_down(&mut self) {
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

    /// Selected family name if (and only if) the palette is in fonts
    /// mode and the selection points at a valid row. Owned `String`
    /// so the caller can mutate the palette state (`set_enabled`) in
    /// the same statement without fighting the borrow checker.
    pub fn get_selected_font(&self) -> Option<String> {
        self.filtered_rows()
            .get(self.selected_index)
            .and_then(|(_, row)| match row {
                PaletteRow::Font { family } => Some((*family).to_owned()),
                PaletteRow::Command { .. } | PaletteRow::Market { .. } => None,
            })
    }

    pub fn get_selected_market_id(&self) -> Option<String> {
        self.filtered_rows()
            .get(self.selected_index)
            .and_then(|(_, row)| match row {
                PaletteRow::Market { id, .. } => Some((*id).to_owned()),
                PaletteRow::Command { .. } | PaletteRow::Font { .. } => None,
            })
    }

    /// Filtered list of rows for the current mode. Both modes share
    /// the same fuzzy-score + sort pipeline so typing behaves
    /// identically in either view.
    fn filtered_rows(&self) -> Vec<(i32, PaletteRow<'_>)> {
        let mut results: Vec<(i32, PaletteRow<'_>)> = match &self.mode {
            PaletteMode::Commands => {
                let has_adaptive = self.has_adaptive_theme;
                COMMANDS
                    .iter()
                    .filter(|cmd| {
                        if cmd.action == PaletteAction::ToggleAppearanceTheme {
                            return has_adaptive;
                        }
                        true
                    })
                    .filter_map(|cmd| {
                        let score = fuzzy_score(&self.query, cmd.title)?;
                        Some((
                            score,
                            PaletteRow::Command {
                                title: cmd.title,
                                shortcut: cmd.shortcut,
                                action: cmd.action,
                            },
                        ))
                    })
                    .collect()
            }
            PaletteMode::Fonts(fonts) => fonts
                .iter()
                .filter_map(|family| {
                    let score = fuzzy_score(&self.query, family)?;
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
                    .filter_map(|candidate| fuzzy_score(&self.query, candidate))
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
        };

        results.sort_by_key(|r| std::cmp::Reverse(r.0));
        results
    }

    /// Returns the palette geometry for drawing and hit-testing. Width and row
    /// count contract before either edge can leave the live viewport.
    fn palette_rect(
        &self,
        window_width: f32,
        window_height: f32,
        scale_factor: f32,
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
            .clamp(1, MAX_VISIBLE_RESULTS);
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
        let (px, py, pw, ph, visible_results) =
            self.palette_rect(window_width, window_height, scale_factor);

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
        let filtered_count = self.filtered_rows().len();
        let actual_index = self.scroll_offset + row;

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

        // UI glyphs are submitted in one pass after every rounded rectangle.
        // Without a modal boundary, labels emitted earlier by window tabs or a
        // pane-local tab rail remain above even an opaque palette surface. The
        // command center is the top-most modal owner:
        // discard earlier UI-label instances, then emit only palette labels.
        // Terminal grid text uses a separate pass and remains safely beneath
        // the palette's opaque blue-black surface.
        sugarloaf.text_mut().clear();

        let (window_width, window_height, scale_factor) = dimensions;

        let (palette_x, palette_y, palette_width, palette_height, visible_results) =
            self.palette_rect(window_width, window_height, scale_factor);
        self.visible_results = visible_results;
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_results {
            self.scroll_offset = self.selected_index + 1 - visible_results;
        }

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

        // Lift the command center above terminal content, then carve a crisp
        // one-pixel Automexia outline around the blue-black glass surface.
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
            9.0,
            INPUT_OUTLINE_COLOR,
            INPUT_BG_COLOR,
            DEPTH_ELEMENT,
            ORDER,
        );
        draw_command_icon(
            sugarloaf,
            CommandIcon::Search,
            input_x + 13.0,
            input_y + 11.0,
            BRAND_CYAN,
            INPUT_BG_COLOR,
        );

        let esc_x = input_x + input_width - ESC_BADGE_WIDTH - 10.0;
        stroke_rounded_rect(
            sugarloaf,
            esc_x,
            input_y + 11.0,
            ESC_BADGE_WIDTH,
            25.0,
            1.0,
            6.0,
            SHORTCUT_OUTLINE_COLOR,
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
            PaletteMode::Commands => "Type a command...",
            PaletteMode::Fonts(_) => "Type a font name...",
            PaletteMode::Market(_) => "Search extensions...",
        };
        let input_text_width = (input_width
            - INPUT_PADDING_X * 2.0
            - INPUT_ICON_WELL
            - ESC_BADGE_WIDTH
            - 18.0)
            .max(0.0);
        let display_text = if self.query.is_empty() {
            elide_end(sugarloaf, placeholder, input_text_width, INPUT_FONT_SIZE)
        } else {
            elide_start(
                sugarloaf,
                self.query.as_str(),
                input_text_width,
                INPUT_FONT_SIZE,
            )
        };
        let text_color = if self.query.is_empty() {
            DIM_TEXT_COLOR
        } else {
            TEXT_COLOR
        };

        let text_x = input_x + INPUT_PADDING_X + INPUT_ICON_WELL;
        let text_y = input_y + (INPUT_HEIGHT - INPUT_FONT_SIZE) / 2.0;
        let input_opts = DrawOpts {
            font_size: INPUT_FONT_SIZE,
            color: color_u8(text_color),
            ..DrawOpts::default()
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
        let filtered = self.filtered_rows();

        for (display_i, (_, row)) in filtered
            .iter()
            .skip(self.scroll_offset)
            .take(visible_results)
            .enumerate()
        {
            let actual_index = self.scroll_offset + display_i;
            let item_y = results_y + RESULT_ITEM_HEIGHT * display_i as f32;
            let is_selected = actual_index == self.selected_index;
            let presentation = row.presentation();

            if is_selected {
                stroke_rounded_rect(
                    sugarloaf,
                    input_x,
                    item_y,
                    input_width,
                    RESULT_ITEM_HEIGHT - 2.0,
                    1.0,
                    8.0,
                    SELECTED_OUTLINE_COLOR,
                    SELECTED_BG_COLOR,
                    DEPTH_ELEMENT,
                    ORDER,
                );
                sugarloaf.rounded_rect(
                    None,
                    input_x + 4.0,
                    item_y + 11.0,
                    2.0,
                    RESULT_ITEM_HEIGHT - 24.0,
                    presentation.accent,
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
            let is_font_row = matches!(row, PaletteRow::Font { .. });
            let trailing_width = if !shortcut.is_empty() {
                let shortcut_opts = DrawOpts {
                    font_size: SHORTCUT_FONT_SIZE,
                    color: color_u8(SHORTCUT_TEXT_COLOR),
                    ..DrawOpts::default()
                };
                sugarloaf.text_mut().measure(shortcut, &shortcut_opts) + 30.0
            } else if is_font_row {
                COPY_ICON_W + 24.0
            } else {
                10.0
            };
            let row_title = elide_end(
                sugarloaf,
                row.title(),
                (input_x + input_width - row_text_x - trailing_width).max(0.0),
                RESULT_FONT_SIZE,
            );
            sugarloaf
                .text_mut()
                .draw(row_text_x, row_text_y, &row_title, &result_opts);

            if !shortcut.is_empty() {
                let shortcut_opts = DrawOpts {
                    font_size: SHORTCUT_FONT_SIZE,
                    color: color_u8(if is_selected {
                        TEXT_COLOR
                    } else {
                        SHORTCUT_TEXT_COLOR
                    }),
                    ..DrawOpts::default()
                };
                let shortcut_width =
                    sugarloaf.text_mut().measure(shortcut, &shortcut_opts);
                let keycap_width = shortcut_width + 18.0;
                let shortcut_x = input_x + input_width - 10.0 - keycap_width;
                let shortcut_y = item_y + 10.0;
                stroke_rounded_rect(
                    sugarloaf,
                    shortcut_x,
                    shortcut_y,
                    keycap_width,
                    24.0,
                    1.0,
                    6.0,
                    SHORTCUT_OUTLINE_COLOR,
                    SHORTCUT_BG_COLOR,
                    DEPTH_ELEMENT + 0.01,
                    ORDER,
                );
                sugarloaf.text_mut().draw(
                    shortcut_x + 9.0,
                    shortcut_y + 6.0,
                    shortcut,
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

        // Scrollbar: shares the terminal scrollbar's visual language
        // (6 px wide, gray semi-transparent, 2 s visibility + 300 ms
        // fade after the last scroll event) via `renderer::scrollbar`.
        // Drawn only when the palette has actually been scrolled —
        // hidden on first open, faded out 2.3 s after the last scroll.
        let total = filtered.len();
        let track_height = visible_results as f32 * RESULT_ITEM_HEIGHT;
        let normalized = if total > visible_results {
            self.scroll_offset as f32 / (total - visible_results) as f32
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
            let opacity = scrollbar::opacity_from_last_scroll(
                self.last_scroll_time,
                false, // palette has no drag interaction
            );
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
        let palette = CommandPalette::new();
        let filtered = palette.filtered_rows();
        // ToggleAppearanceTheme is hidden when has_adaptive_theme is false
        assert_eq!(filtered.len(), COMMANDS.len() - 1);
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
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.title(), "Quit");
    }

    #[test]
    fn market_command_uses_plain_label() {
        let market = COMMANDS
            .iter()
            .find(|command| command.action == PaletteAction::OpenMarket)
            .expect("market command");
        assert_eq!(market.title, "market");
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
        let palette = CommandPalette::new();
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

        assert_eq!(clone_right.shortcut, "Ctrl+R");
        assert_eq!(clone_down.shortcut, "Ctrl+D");
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
    fn visible_palette_shortcuts_are_unique() {
        let mut shortcuts = std::collections::HashMap::new();
        for command in COMMANDS
            .iter()
            .filter(|command| !command.shortcut.is_empty())
        {
            assert!(
                shortcuts.insert(command.shortcut, command.title).is_none(),
                "duplicate palette shortcut {}",
                command.shortcut
            );
        }
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
    // the scrollbar only surfaces after the user actually scrolls, and
    // resets when the list reshapes.

    #[test]
    fn scrollbar_hidden_until_first_scroll() {
        // Long list, palette just opened — no scroll event has happened,
        // so the fade timer is `None` and the scrollbar stays invisible
        // despite the list being taller than the visible window.
        let mut palette = CommandPalette::new();
        palette.enter_fonts_mode((0..50).map(|i| format!("Family {i:02}")).collect());
        assert!(palette.last_scroll_time.is_none());
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
}
