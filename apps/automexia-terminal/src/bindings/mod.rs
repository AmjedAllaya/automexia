// Binding<T>, MouseAction, BindingMode, Action, default_key_bindings and including their comments
// was originally taken from https://github.com/alacritty/alacritty/blob/e35e5ad14fce8456afdd89f2b392b9924bb27471/alacritty/src/config/bindings.rs
// which is licensed under Apache 2.0 license.

pub mod kitty_keyboard;
pub mod registry;
pub(crate) mod shortcut;

use crate::crosswords::vi_mode::ViMotion;
use crate::crosswords::Mode;
use crate::selection::SelectionMotion;
use bitflags::bitflags;
use rio_backend::config::bindings::KeyBinding as ConfigKeyBinding;
use rio_backend::config::keyboard::Keyboard as ConfigKeyboard;
use rio_window::event::MouseButton;
use rio_window::keyboard::Key::*;
use rio_window::keyboard::NamedKey::*;
use rio_window::keyboard::{Key, KeyLocation, ModifiersState, PhysicalKey};
use std::fmt::Debug;
// use rio_window::platform::scancode::PhysicalKeyExtScancode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontSizeAction {
    Increase,
    Decrease,
    Reset,
}

/// Mouse binding specific actions.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MouseAction {
    /// Expand the selection to the current mouse cursor position.
    ExpandSelection,
}

impl From<MouseAction> for Action {
    fn from(action: MouseAction) -> Self {
        Self::Mouse(action)
    }
}

/// Search mode specific actions.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SearchAction {
    /// Move the focus to the next search match.
    SearchFocusNext,
    /// Move the focus to the previous search match.
    SearchFocusPrevious,
    /// Confirm the active search.
    SearchConfirm,
    /// Cancel the active search.
    SearchCancel,
    /// Reset the search regex.
    SearchClear,
    /// Delete the last word in the search regex.
    SearchDeleteWord,
    /// Go to the previous regex in the search history.
    SearchHistoryPrevious,
    /// Go to the next regex in the search history.
    SearchHistoryNext,
}

impl From<SearchAction> for Action {
    fn from(action: SearchAction) -> Self {
        Self::Search(action)
    }
}

impl From<SelectionMotion> for Action {
    fn from(motion: SelectionMotion) -> Self {
        Self::ExtendSelection(motion)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding<T> {
    /// Modifier keys required to activate binding.
    pub mods: ModifiersState,

    /// String to send to PTY if mods and mode match.
    pub action: Action,

    /// Binding mode required to activate binding.
    pub mode: BindingMode,

    /// Excluded binding modes where the binding won't be activated.
    pub notmode: BindingMode,

    /// This property is used as part of the trigger detection code.
    ///
    /// For example, this might be a key like "G", or a mouse button.
    pub trigger: T,
}

impl<T: Eq> Binding<T> {
    #[inline]
    pub fn is_triggered_by(
        &self,
        mode: BindingMode,
        mods: ModifiersState,
        input: &T,
    ) -> bool {
        // Check input first since bindings are stored in one big list. This is
        // the most likely item to fail so prioritizing it here allows more
        // checks to be short circuited.
        self.trigger == *input
            && self.mods == mods
            && mode.contains(self.mode.clone())
            && !mode.intersects(self.notmode.clone())
    }

    #[inline]
    pub fn triggers_match(&self, binding: &Binding<T>) -> bool {
        // Check the binding's key and modifiers.
        if self.trigger != binding.trigger || self.mods != binding.mods {
            return false;
        }

        let selfmode = if self.mode.is_empty() {
            BindingMode::all()
        } else {
            self.mode.clone()
        };
        let bindingmode = if binding.mode.is_empty() {
            BindingMode::all()
        } else {
            binding.mode.clone()
        };

        if !selfmode.intersects(bindingmode) {
            return false;
        }

        // The bindings are never active at the same time when the required modes of one binding
        // are part of the forbidden bindings of the other.
        if self.mode.intersects(binding.notmode.clone())
            || binding.mode.intersects(self.notmode.clone())
        {
            return false;
        }

        true
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum BindingKey {
    #[allow(dead_code)]
    Scancode(PhysicalKey),
    Keycode {
        key: Key,
        location: KeyLocation,
    },
}

pub type KeyBinding = Binding<BindingKey>;
pub type KeyBindings = Vec<KeyBinding>;

/// Bindings that are triggered by a mouse button.
pub type MouseBinding = Binding<MouseButton>;

bitflags! {
    /// Modes available for key bindings.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct BindingMode: u8 {
        const APP_CURSOR          = 0b0000_0001;
        const APP_KEYPAD          = 0b0000_0010;
        const ALT_SCREEN          = 0b0000_0100;
        const VI                  = 0b0000_1000;
        const SEARCH              = 0b0001_0000;
        const DISAMBIGUATE_KEYS   = 0b0010_0000;
        const ALL_KEYS_AS_ESC     = 0b0100_0000;
    }
}

impl BindingMode {
    pub fn new(mode: &Mode, search: bool) -> BindingMode {
        let mut binding_mode = BindingMode::empty();
        binding_mode.set(BindingMode::APP_CURSOR, mode.contains(Mode::APP_CURSOR));
        binding_mode.set(BindingMode::APP_KEYPAD, mode.contains(Mode::APP_KEYPAD));
        binding_mode.set(BindingMode::ALT_SCREEN, mode.contains(Mode::ALT_SCREEN));
        binding_mode.set(BindingMode::SEARCH, search);
        binding_mode.set(
            BindingMode::DISAMBIGUATE_KEYS,
            mode.contains(Mode::DISAMBIGUATE_ESC_CODES),
        );
        binding_mode.set(
            BindingMode::ALL_KEYS_AS_ESC,
            mode.contains(Mode::REPORT_ALL_KEYS_AS_ESC),
        );
        binding_mode.set(BindingMode::VI, mode.contains(Mode::VI));
        binding_mode
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(unused)]
pub enum Program {
    Just(String),
    WithArgs { program: String, args: Vec<String> },
}

impl Program {
    pub fn program(&self) -> &str {
        match self {
            Program::Just(program) => program,
            Program::WithArgs { program, .. } => program,
        }
    }

    pub fn args(&self) -> &[String] {
        match self {
            Program::Just(_) => &[],
            Program::WithArgs { args, .. } => args,
        }
    }
}

impl From<String> for Action {
    fn from(action: String) -> Action {
        let action = action.to_lowercase();

        let action_from_string = match action.as_str() {
            "paste" => Some(Action::Paste),
            "quit" => Some(Action::Quit),
            "copy" => Some(Action::Copy),
            "selectall" => Some(Action::SelectAll),
            "extendselectionleft" => Some(Action::ExtendSelection(SelectionMotion::Left)),
            "extendselectionright" => {
                Some(Action::ExtendSelection(SelectionMotion::Right))
            }
            "extendselectionup" => Some(Action::ExtendSelection(SelectionMotion::Up)),
            "extendselectiondown" => Some(Action::ExtendSelection(SelectionMotion::Down)),
            "extendselectionwordleft" => {
                Some(Action::ExtendSelection(SelectionMotion::WordLeft))
            }
            "extendselectionwordright" => {
                Some(Action::ExtendSelection(SelectionMotion::WordRight))
            }
            "searchforward" => Some(Action::SearchForward),
            "searchbackward" => Some(Action::SearchBackward),
            "searchglobalforward" => Some(Action::SearchGlobalForward),
            "searchglobalbackward" => Some(Action::SearchGlobalBackward),
            "searchconfirm" => Some(Action::Search(SearchAction::SearchConfirm)),
            "searchcancel" => Some(Action::Search(SearchAction::SearchCancel)),
            "searchclear" => Some(Action::Search(SearchAction::SearchClear)),
            "searchfocusnext" => Some(Action::Search(SearchAction::SearchFocusNext)),
            "searchfocusprevious" => {
                Some(Action::Search(SearchAction::SearchFocusPrevious))
            }
            "searchdeleteword" => Some(Action::Search(SearchAction::SearchDeleteWord)),
            "searchhistorynext" => Some(Action::Search(SearchAction::SearchHistoryNext)),
            "searchhistoryprevious" => {
                Some(Action::Search(SearchAction::SearchHistoryPrevious))
            }
            "clearhistory" => Some(Action::ClearHistory),
            "clearscreen" => Some(Action::ClearScreen),
            "resetfontsize" => Some(Action::ResetFontSize),
            "increasefontsize" => Some(Action::IncreaseFontSize),
            "decreasefontsize" => Some(Action::DecreaseFontSize),
            "createwindow" => Some(Action::WindowCreateNew),
            "closewindow" => Some(Action::WindowClose),
            "reloadconfig" => Some(Action::ReloadConfig),
            "togglequake" => Some(Action::ToggleQuake),
            "scrolltoprevprompt" => Some(Action::ScrollToPrevPrompt),
            "scrolltonextprompt" => Some(Action::ScrollToNextPrompt),
            "createtab" => Some(Action::TabCreateNew),
            "createlocaltab" => Some(Action::LocalTabCreateNew),
            "movecurrenttabtoprev" => Some(Action::MoveCurrentTabToPrev),
            "movecurrenttabtonext" => Some(Action::MoveCurrentTabToNext),
            "closetab" => Some(Action::TabCloseCurrent),
            "closesplitortab" => Some(Action::CloseCurrentSplitOrTab),
            "closeunfocusedtabs" => Some(Action::TabCloseUnfocused),
            "openconfigeditor" => Some(Action::ConfigEditor),
            "opensettings" => Some(Action::OpenSettings),
            "selectprevtab" => Some(Action::SelectPrevTab),
            "selectnexttab" => Some(Action::SelectNextTab),
            "selectprevlocaltab" => Some(Action::SelectPrevLocalTab),
            "selectnextlocaltab" => Some(Action::SelectNextLocalTab),
            "selectlasttab" => Some(Action::SelectLastTab),
            "receivechar" => Some(Action::ReceiveChar),
            "scrollpageup" => Some(Action::ScrollPageUp),
            "scrollpagedown" => Some(Action::ScrollPageDown),
            "scrollhalfpageup" => Some(Action::ScrollHalfPageUp),
            "scrollhalfpagedown" => Some(Action::ScrollHalfPageDown),
            "scrolltotop" => Some(Action::ScrollToTop),
            "scrolltobottom" => Some(Action::ScrollToBottom),
            "splitright" => Some(Action::SplitRight),
            "splitdown" => Some(Action::SplitDown),
            "clonesplitright" => Some(Action::CloneSplitRight),
            "clonesplitdown" => Some(Action::CloneSplitDown),
            "selectnextsplit" => Some(Action::SelectNextSplit),
            "selectprevsplit" => Some(Action::SelectPrevSplit),
            "selectpaneleft" => Some(Action::SelectPaneLeft),
            "selectpaneright" => Some(Action::SelectPaneRight),
            "selectpaneup" => Some(Action::SelectPaneUp),
            "selectpanedown" => Some(Action::SelectPaneDown),
            "selectnextsplitortab" => Some(Action::SelectNextSplitOrTab),
            "selectprevsplitortab" => Some(Action::SelectPrevSplitOrTab),
            "movedividerup" => Some(Action::MoveDividerUp),
            "movedividerdown" => Some(Action::MoveDividerDown),
            "movedividerleft" => Some(Action::MoveDividerLeft),
            "movedividerright" => Some(Action::MoveDividerRight),
            "togglevimode" => Some(Action::ToggleViMode),
            "toggleappearancetheme" => Some(Action::ToggleAppearanceTheme),
            "togglefullscreen" => Some(Action::ToggleFullscreen),
            "opencommandpalette" => Some(Action::OpenCommandPalette),
            "openconnectionhub" => Some(Action::OpenConnectionHub),
            "openactioncenter" => Some(Action::OpenActionCenter),
            "openextensionmarketplace" => Some(Action::OpenExtensionMarketplace),
            "openfontbrowser" => Some(Action::OpenFontBrowser),
            "previewselectedimage" => Some(Action::PreviewSelectedImage),
            "viewtableoutput" => Some(Action::ViewTableOutput),
            "none" => Some(Action::None),
            _ => None,
        };

        if action_from_string.is_some() {
            return action_from_string.unwrap_or(Action::None);
        }

        let re = regex::Regex::new(r"selecttab\(([^()]+)\)").unwrap();
        for capture in re.captures_iter(&action) {
            if let Some(matched) = capture.get(1) {
                let matched_string = matched.as_str().to_string();
                let parsed_matched_string: usize = matched_string.parse().unwrap_or(0);
                return Action::SelectTab(parsed_matched_string);
            }
        }

        let re = regex::Regex::new(r"run\(([^()]+)\)").unwrap();
        for capture in re.captures_iter(&action) {
            if let Some(matched) = capture.get(1) {
                let matched_string = matched.as_str().to_string();
                if matched_string.contains(' ') {
                    let mut vec_program_with_args: Vec<String> =
                        matched_string.split(' ').map(|s| s.to_string()).collect();
                    if vec_program_with_args.is_empty() {
                        continue;
                    }

                    let program = vec_program_with_args[0].to_string();
                    vec_program_with_args.remove(0);

                    return Action::Run(Program::WithArgs {
                        program,
                        args: vec_program_with_args,
                    });
                } else {
                    return Action::Run(Program::Just(matched_string));
                }
            }
        }

        let re = regex::Regex::new(r"scroll\(([^()]+)\)").unwrap();
        for capture in re.captures_iter(&action) {
            if let Some(matched) = capture.get(1) {
                let matched_string = matched.as_str().to_string();
                let parsed_matched_string: i32 = matched_string.parse().unwrap_or(1);
                return Action::Scroll(parsed_matched_string);
            }
        }

        Action::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Write an escape sequence.
    Esc(String),

    /// Run given command.
    Run(Program),

    /// Scroll
    Scroll(i32),

    /// Activate hint mode with the given hint index
    Hint(std::rc::Rc<rio_backend::config::hints::Hint>),

    // Move vi mode cursor.
    ViMotion(ViMotion),

    // Perform vi mode action.
    Vi(ViAction),
    /// Perform mouse binding exclusive action.
    Mouse(MouseAction),

    /// Paste contents of system clipboard.
    Paste,

    /// Store current selection into clipboard.
    Copy,

    #[cfg(not(any(target_os = "macos", windows)))]
    #[allow(dead_code)]
    /// Store current selection into selection buffer.
    CopySelection,

    /// Paste contents of selection buffer.
    PasteSelection,

    /// Increase font size.
    IncreaseFontSize,

    /// Decrease font size.
    DecreaseFontSize,

    /// Reset font size to the config value.
    ResetFontSize,

    /// Scroll exactly one page up.
    ScrollPageUp,

    /// Scroll exactly one page down.
    ScrollPageDown,

    /// Scroll half a page up.
    ScrollHalfPageUp,

    /// Scroll half a page down.
    ScrollHalfPageDown,

    /// Scroll all the way to the top.
    ScrollToTop,

    /// Scroll all the way to the bottom.
    ScrollToBottom,

    /// Clear the display buffer(s) to remove history.
    ClearHistory,

    /// Clear the visible screen and all scrollback.
    ClearScreen,

    /// Hide the Automexia window.
    #[allow(dead_code)]
    Hide,

    /// Hide all windows other than Automexia on macOS.
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    HideOtherApplications,

    /// Minimize the Automexia window.
    #[allow(dead_code)]
    Minimize,

    /// Quit Automexia.
    Quit,

    /// Clear warning and error notices.
    ClearLogNotice,

    /// Spawn a new instance of Automexia.
    #[allow(dead_code)]
    SpawnNewInstance,

    /// Create a new Automexia window.
    #[allow(dead_code)]
    WindowCreateNew,

    /// Close only the current Automexia window.
    WindowClose,

    /// Reload Automexia's configuration from disk.
    ReloadConfig,

    /// Create config editor.
    ConfigEditor,
    OpenSettings,

    /// Create a new Automexia tab.
    TabCreateNew,

    /// Create an independent tab inside the selected split pane.
    LocalTabCreateNew,

    /// Move current tab to previous slot.
    MoveCurrentTabToPrev,

    /// Move current tab to next slot.
    MoveCurrentTabToNext,

    /// Switch to next tab.
    SelectNextTab,

    /// Switch to prev tab.
    SelectPrevTab,

    /// Switch to the next independent tab inside the selected pane.
    SelectNextLocalTab,

    /// Switch to the previous independent tab inside the selected pane.
    SelectPrevLocalTab,

    /// Close tab.
    TabCloseCurrent,

    CloseCurrentSplitOrTab,

    /// Close all other tabs (leave only the current tab).
    TabCloseUnfocused,

    /// Toggle fullscreen.
    #[allow(dead_code)]
    ToggleFullscreen,

    /// Toggle maximized.
    #[allow(dead_code)]
    ToggleMaximized,

    /// Toggle simple fullscreen on macOS.
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    ToggleSimpleFullscreen,

    /// Clear active selection.
    ClearSelection,

    /// Select everything, including the scrollback history.
    SelectAll,

    /// Extend the terminal-owned selection without sending input to the PTY.
    ExtendSelection(SelectionMotion),

    /// Show or hide the quake-style dropdown window. Also registered
    /// as a system-wide hotkey when bound in `[bindings]`.
    ToggleQuake,

    /// Scroll so the previous OSC 133 prompt is at the top of the
    /// viewport. Requires shell integration emitting semantic prompts.
    ScrollToPrevPrompt,

    /// Scroll so the next OSC 133 prompt is at the top of the viewport.
    ScrollToNextPrompt,

    /// Toggle vi mode.
    ToggleViMode,

    /// Toggle appearance theme (dark/light).
    ToggleAppearanceTheme,

    /// Open the application-owned, read-only Connection Hub.
    OpenConnectionHub,

    /// Open the application-owned action search and review surface.
    OpenActionCenter,

    /// Open the extension marketplace browser.
    OpenExtensionMarketplace,

    /// Open the registered font-family browser.
    OpenFontBrowser,

    // Tab selections
    SelectTab(usize),
    SelectLastTab,

    Search(SearchAction),
    /// Start a forward buffer search.
    SearchForward,

    /// Start a backward search in the selected pane.
    SearchBackward,

    /// Start a forward search across every visible pane.
    SearchGlobalForward,

    /// Start a backward search across every visible pane.
    SearchGlobalBackward,

    /// Split horizontally
    SplitRight,

    /// Split vertically
    SplitDown,

    /// Create an independent clone of the active session in a right split.
    CloneSplitRight,

    /// Create an independent clone of the active session in a lower split.
    CloneSplitDown,

    /// Select next split
    SelectNextSplit,

    /// Select previous split
    SelectPrevSplit,

    /// Select the nearest pane to the left.
    SelectPaneLeft,

    /// Select the nearest pane to the right.
    SelectPaneRight,

    /// Select the nearest pane above.
    SelectPaneUp,

    /// Select the nearest pane below.
    SelectPaneDown,

    /// Select next split if available if not next tab
    SelectNextSplitOrTab,

    /// Select previous split if available if not previous tab
    SelectPrevSplitOrTab,

    /// Move divider up
    MoveDividerUp,

    /// Move divider down
    MoveDividerDown,

    /// Move divider left
    MoveDividerLeft,

    /// Move divider right
    MoveDividerRight,

    /// Preview the selected or pointer-targeted local raster image.
    PreviewSelectedImage,
    ViewTableOutput,

    /// Toggle the command palette overlay.
    OpenCommandPalette,

    /// Allow receiving char input.
    ReceiveChar,

    /// No action.
    None,
}

impl From<&'static str> for Action {
    fn from(s: &'static str) -> Action {
        Action::Esc(s.into())
    }
}

impl From<ViMotion> for Action {
    fn from(motion: ViMotion) -> Self {
        Self::ViMotion(motion)
    }
}

impl From<ViAction> for Action {
    fn from(action: ViAction) -> Self {
        Self::Vi(action)
    }
}

/// Vi mode specific actions.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ViAction {
    /// Toggle normal vi selection.
    ToggleNormalSelection,
    /// Toggle line vi selection.
    ToggleLineSelection,
    /// Toggle block vi selection.
    ToggleBlockSelection,
    /// Toggle semantic vi selection.
    ToggleSemanticSelection,
    /// Centers the screen around the vi mode cursor.
    CenterAroundViCursor,
}

macro_rules! bindings {
    (
        $ty:ident;
        $(
            $key:expr
            $(=>$location:expr)?
            $(,$mods:expr)*
            $(,+$mode:expr)*
            $(,~$notmode:expr)*
            ;$action:expr
        );*
        $(;)*
    ) => {{
        let mut v = Vec::new();

        $(
            let mut _mods = ModifiersState::empty();
            $(_mods = $mods;)*
            let mut _mode = BindingMode::empty();
            $(_mode.insert($mode);)*
            let mut _notmode = BindingMode::empty();
            $(_notmode.insert($notmode);)*

            v.push($ty {
                trigger: trigger!($ty, $key, $($location)?),
                mods: _mods,
                mode: _mode,
                notmode: _notmode,
                action: $action.into(),
            });
        )*

        v
    }};
}

macro_rules! trigger {
    (KeyBinding, $key:literal, $location:expr) => {{
        BindingKey::Keycode {
            key: Character($key.into()),
            location: $location,
        }
    }};
    (KeyBinding, $key:literal,) => {{
        BindingKey::Keycode {
            key: Character($key.into()),
            location: KeyLocation::Standard,
        }
    }};
    (KeyBinding, $key:expr,) => {{
        BindingKey::Keycode {
            key: $key,
            location: KeyLocation::Standard,
        }
    }};
    ($ty:ident, $key:expr,) => {{
        $key
    }};
}

pub fn default_mouse_bindings() -> Vec<MouseBinding> {
    bindings!(
        MouseBinding;
        MouseButton::Right,  ~BindingMode::VI;         Action::Paste;
        MouseButton::Right,   ModifiersState::CONTROL; MouseAction::ExpandSelection;
        MouseButton::Middle, ~BindingMode::VI;         Action::PasteSelection;
    )
}

pub fn default_key_bindings(config: &rio_backend::config::Config) -> Vec<KeyBinding> {
    key_bindings_with_platform(config, platform_key_bindings)
}

#[cfg(test)]
pub(crate) fn test_platform_defaults(
    config: &rio_backend::config::Config,
    platform: automexia_keybindings::PlatformFamily,
) -> Vec<KeyBinding> {
    use automexia_keybindings::PlatformFamily;
    key_bindings_with_platform(config, |navigation, splits, keyboard| match platform {
        PlatformFamily::Windows => automexia_windows_key_bindings(navigation, splits),
        PlatformFamily::LinuxBsd => automexia_unix_key_bindings(navigation, splits),
        PlatformFamily::Macos => {
            automexia_macos_key_bindings(navigation, splits, keyboard)
        }
    })
}

fn key_bindings_with_platform(
    config: &rio_backend::config::Config,
    platform: impl FnOnce(bool, bool, ConfigKeyboard) -> Vec<KeyBinding>,
) -> Vec<KeyBinding> {
    if config.keyboard.binding_profile != automexia_keybindings::ProfileId::Automexia {
        return shortcut::apply_classic(
            &config.bindings.ui_shortcuts,
            config_key_bindings(config.bindings.keys.to_owned(), Vec::new()),
        );
    }
    let mut bindings = bindings!(
        KeyBinding;
        Key::Named(Copy);  Action::Copy;
        Key::Named(F7), ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH; Action::ViewTableOutput;
        Key::Named(Copy),  +BindingMode::VI; Action::ClearSelection;
        Key::Named(Paste), ~BindingMode::VI; Action::Paste;
        Key::Named(ArrowLeft),  ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH; SelectionMotion::Left;
        Key::Named(ArrowRight), ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH; SelectionMotion::Right;
        Key::Named(ArrowUp),    ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH; SelectionMotion::Up;
        Key::Named(ArrowDown),  ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH; SelectionMotion::Down;
        Key::Named(ArrowLeft),  ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH; SelectionMotion::WordLeft;
        Key::Named(ArrowRight), ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH; SelectionMotion::WordRight;
        Key::Character("l".into()), ModifiersState::CONTROL; Action::ClearLogNotice;
        "l",  ModifiersState::CONTROL, ~BindingMode::VI; Action::Esc("\x0c".into());
        Key::Named(Home),     ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN; Action::ScrollToTop;
        Key::Named(End),      ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN; Action::ScrollToBottom;
        Key::Named(PageUp),   ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN; Action::ScrollPageUp;
        Key::Named(PageDown), ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN; Action::ScrollPageDown;
        Key::Named(Home),  +BindingMode::APP_CURSOR, ~BindingMode::VI;
            Action::Esc("\x1bOH".into());
        Key::Named(End),   +BindingMode::APP_CURSOR, ~BindingMode::VI;
            Action::Esc("\x1bOF".into());
        Key::Named(ArrowUp),    +BindingMode::APP_CURSOR, ~BindingMode::VI;
            Action::Esc("\x1bOA".into());
        Key::Named(ArrowDown),  +BindingMode::APP_CURSOR, ~BindingMode::VI;
            Action::Esc("\x1bOB".into());
        Key::Named(ArrowRight), +BindingMode::APP_CURSOR, ~BindingMode::VI;
            Action::Esc("\x1bOC".into());
        Key::Named(ArrowLeft),  +BindingMode::APP_CURSOR, ~BindingMode::VI;
            Action::Esc("\x1bOD".into());

        // VI Mode
        Key::Named(Space), ModifiersState::ALT | ModifiersState::SHIFT; Action::ToggleViMode;
        "/", +BindingMode::VI, ~BindingMode::SEARCH; Action::SearchForward;
        "n", +BindingMode::VI, ~BindingMode::SEARCH; SearchAction::SearchFocusNext;
        "n",  ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH; SearchAction::SearchFocusPrevious;
        Key::Named(Enter), +BindingMode::SEARCH, ~BindingMode::VI; SearchAction::SearchFocusNext;
        Key::Named(Enter), +BindingMode::SEARCH, +BindingMode::VI; SearchAction::SearchConfirm;
        Key::Named(Escape), +BindingMode::SEARCH; SearchAction::SearchCancel;
        Key::Named(Enter), ModifiersState::SHIFT, +BindingMode::SEARCH, ~BindingMode::VI; SearchAction::SearchFocusPrevious;
        "i", +BindingMode::VI, ~BindingMode::SEARCH; Action::ToggleViMode;
        "c", ModifiersState::CONTROL, +BindingMode::VI; Action::ToggleViMode;
        Key::Named(Escape), +BindingMode::VI; Action::ClearSelection;
        "i", +BindingMode::VI, ~BindingMode::SEARCH; Action::ScrollToBottom;
        "g", +BindingMode::VI, ~BindingMode::SEARCH; Action::ScrollToTop;
        "g", ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH; Action::ScrollToBottom;
        "b", ModifiersState::CONTROL, +BindingMode::VI, ~BindingMode::SEARCH; Action::ScrollPageUp;
        "f", ModifiersState::CONTROL, +BindingMode::VI, ~BindingMode::SEARCH; Action::ScrollPageDown;
        "u", ModifiersState::CONTROL, +BindingMode::VI, ~BindingMode::SEARCH; Action::ScrollHalfPageUp;
        "d", ModifiersState::CONTROL, +BindingMode::VI, ~BindingMode::SEARCH; Action::ScrollHalfPageDown;
        "y", ModifiersState::CONTROL,  +BindingMode::VI, ~BindingMode::SEARCH; Action::Scroll(1);
        "e", ModifiersState::CONTROL,  +BindingMode::VI, ~BindingMode::SEARCH; Action::Scroll(-1);
        "y", +BindingMode::VI, ~BindingMode::SEARCH; Action::Copy;
        "y", +BindingMode::VI, ~BindingMode::SEARCH; Action::ClearSelection;
        "v", +BindingMode::VI, ~BindingMode::SEARCH; ViAction::ToggleNormalSelection;
        "v", ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH; ViAction::ToggleLineSelection;
        "v", ModifiersState::CONTROL, +BindingMode::VI, ~BindingMode::SEARCH; ViAction::ToggleBlockSelection;
        "v", ModifiersState::ALT, +BindingMode::VI, ~BindingMode::SEARCH; ViAction::ToggleSemanticSelection;
        "z", +BindingMode::VI, ~BindingMode::SEARCH; ViAction::CenterAroundViCursor;
        "k", +BindingMode::VI, ~BindingMode::SEARCH; ViMotion::Up;
        "j", +BindingMode::VI, ~BindingMode::SEARCH; ViMotion::Down;
        "h", +BindingMode::VI, ~BindingMode::SEARCH; ViMotion::Left;
        "l", +BindingMode::VI, ~BindingMode::SEARCH; ViMotion::Right;
        Key::Named(ArrowUp), +BindingMode::VI; ViMotion::Up;
        Key::Named(ArrowDown), +BindingMode::VI; ViMotion::Down;
        Key::Named(ArrowLeft), +BindingMode::VI; ViMotion::Left;
        Key::Named(ArrowRight), +BindingMode::VI; ViMotion::Right;
        Key::Named(ArrowUp), ModifiersState::SUPER, ~BindingMode::VI; Action::None;
        Key::Named(ArrowDown), ModifiersState::SUPER, ~BindingMode::VI; Action::None;
        Key::Named(ArrowLeft), ModifiersState::SUPER, ~BindingMode::VI; Action::None;
        Key::Named(ArrowRight), ModifiersState::SUPER, ~BindingMode::VI; Action::None;
        "0",                          +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::First;
        "4",   ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::Last;
        "6",   ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::FirstOccupied;
        "h",      ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::High;
        "m",      ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::Middle;
        "l",      ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::Low;
        "b",                             +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::SemanticLeft;
        "w",                             +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::SemanticRight;
        "e",                             +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::SemanticRightEnd;
        "b",      ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::WordLeft;
        "w",      ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::WordRight;
        "e",      ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::WordRightEnd;
        "5",   ModifiersState::SHIFT, +BindingMode::VI, ~BindingMode::SEARCH;
            ViMotion::Bracket;
    );

    bindings.extend(bindings!(
        KeyBinding;
        Key::Named(ArrowUp), ~BindingMode::APP_CURSOR, ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC; Action::Esc("\x1b[A".into());
        Key::Named(ArrowDown), ~BindingMode::APP_CURSOR, ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC; Action::Esc("\x1b[B".into());
        Key::Named(ArrowRight), ~BindingMode::APP_CURSOR, ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC; Action::Esc("\x1b[C".into());
        Key::Named(ArrowLeft),  ~BindingMode::APP_CURSOR, ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC; Action::Esc("\x1b[D".into());
        Key::Named(Insert),     ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1b[2~".into());
        Key::Named(Delete),     ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1b[3~".into());
        Key::Named(PageUp),     ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1b[5~".into());
        Key::Named(PageDown),   ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1b[6~".into());
        Key::Named(Backspace),  ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC; Action::Esc("\x7f".into());
        Key::Named(Backspace), ModifiersState::ALT,     ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1b\x7f".into());
        Key::Named(Backspace), ModifiersState::SHIFT,   ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x7f".into());
        Key::Named(F1), ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1bOP".into());
        Key::Named(F2), ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1bOQ".into());
        Key::Named(F3), ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1bOR".into());
        Key::Named(F4), ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1bOS".into());
        Key::Named(Tab),       ModifiersState::SHIFT,   ~BindingMode::VI,   ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1b[Z".into());
        Key::Named(Tab),       ModifiersState::SHIFT | ModifiersState::ALT, ~BindingMode::VI, ~BindingMode::SEARCH, ~BindingMode::ALL_KEYS_AS_ESC, ~BindingMode::DISAMBIGUATE_KEYS; Action::Esc("\x1b\x1b[Z".into());
    ));

    bindings.extend(platform(
        config.navigation.has_navigation_key_bindings(),
        config.navigation.use_split,
        config.keyboard.clone(),
    ));

    // Add hint bindings
    bindings.extend(create_hint_bindings(&config.hints.rules));

    shortcut::apply_classic(
        &config.bindings.ui_shortcuts,
        config_key_bindings(config.bindings.keys.to_owned(), bindings),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModeWrapper {
    pub mode: BindingMode,
    pub not_mode: BindingMode,
}

#[inline]
fn convert(config_key_binding: ConfigKeyBinding) -> Result<KeyBinding, String> {
    let (key, location) = if config_key_binding.key.chars().count() == 1 {
        (
            Key::Character(config_key_binding.key.to_lowercase().into()),
            KeyLocation::Standard,
        )
    } else {
        match config_key_binding.key.to_lowercase().as_str() {
            "home" => (Key::Named(Home), KeyLocation::Standard),
            "space" => (Key::Named(Space), KeyLocation::Standard),
            "delete" => (Key::Named(Delete), KeyLocation::Standard),
            "esc" | "escape" => (Key::Named(Escape), KeyLocation::Standard),
            "insert" => (Key::Named(Insert), KeyLocation::Standard),
            "pageup" => (Key::Named(PageUp), KeyLocation::Standard),
            "pagedown" => (Key::Named(PageDown), KeyLocation::Standard),
            "end" => (Key::Named(End), KeyLocation::Standard),
            "up" => (Key::Named(ArrowUp), KeyLocation::Standard),
            "back" | "backspace" => (Key::Named(Backspace), KeyLocation::Standard),
            "down" => (Key::Named(ArrowDown), KeyLocation::Standard),
            "left" => (Key::Named(ArrowLeft), KeyLocation::Standard),
            "right" => (Key::Named(ArrowRight), KeyLocation::Standard),
            "@" => (Key::Character("@".into()), KeyLocation::Standard),
            "colon" => (Key::Character(":".into()), KeyLocation::Standard),
            "." => (Key::Character(".".into()), KeyLocation::Standard),
            "return" | "enter" => (Key::Named(Enter), KeyLocation::Standard),
            "f1" => (Key::Named(F1), KeyLocation::Standard),
            "f2" => (Key::Named(F2), KeyLocation::Standard),
            "f3" => (Key::Named(F3), KeyLocation::Standard),
            "f4" => (Key::Named(F4), KeyLocation::Standard),
            "f5" => (Key::Named(F5), KeyLocation::Standard),
            "f6" => (Key::Named(F6), KeyLocation::Standard),
            "f7" => (Key::Named(F7), KeyLocation::Standard),
            "f8" => (Key::Named(F8), KeyLocation::Standard),
            "f9" => (Key::Named(F9), KeyLocation::Standard),
            "f10" => (Key::Named(F10), KeyLocation::Standard),
            "f11" => (Key::Named(F11), KeyLocation::Standard),
            "f12" => (Key::Named(F12), KeyLocation::Standard),
            "f13" => (Key::Named(F13), KeyLocation::Standard),
            "f14" => (Key::Named(F14), KeyLocation::Standard),
            "f15" => (Key::Named(F15), KeyLocation::Standard),
            "f16" => (Key::Named(F16), KeyLocation::Standard),
            "f17" => (Key::Named(F17), KeyLocation::Standard),
            "f18" => (Key::Named(F18), KeyLocation::Standard),
            "f19" => (Key::Named(F19), KeyLocation::Standard),
            "f20" => (Key::Named(F20), KeyLocation::Standard),
            "[" => (Key::Character("[".into()), KeyLocation::Standard),
            "]" => (Key::Character("]".into()), KeyLocation::Standard),
            "{" => (Key::Character("{".into()), KeyLocation::Standard),
            "}" => (Key::Character("}".into()), KeyLocation::Standard),
            ";" => (Key::Character(";".into()), KeyLocation::Standard),
            "\\" => (Key::Character("\\".into()), KeyLocation::Standard),
            "+" => (Key::Character("+".into()), KeyLocation::Standard),
            "," => (Key::Character(",".into()), KeyLocation::Standard),
            "/" => (Key::Character("/".into()), KeyLocation::Standard),
            "=" => (Key::Character("=".into()), KeyLocation::Standard),
            "-" => (Key::Character("-".into()), KeyLocation::Standard),
            "*" => (Key::Character("*".into()), KeyLocation::Standard),
            "1" => (Key::Character("1".into()), KeyLocation::Standard),
            "2" => (Key::Character("2".into()), KeyLocation::Standard),
            "3" => (Key::Character("3".into()), KeyLocation::Standard),
            "4" => (Key::Character("4".into()), KeyLocation::Standard),
            "5" => (Key::Character("5".into()), KeyLocation::Standard),
            "6" => (Key::Character("6".into()), KeyLocation::Standard),
            "7" => (Key::Character("7".into()), KeyLocation::Standard),
            "8" => (Key::Character("8".into()), KeyLocation::Standard),
            "9" => (Key::Character("9".into()), KeyLocation::Standard),
            "0" => (Key::Character("0".into()), KeyLocation::Standard),

            // Special case numpad.
            "numpadenter" => (Key::Named(Enter), KeyLocation::Numpad),
            "numpadadd" => (Key::Character("+".into()), KeyLocation::Numpad),
            "numpadcomma" => (Key::Character(",".into()), KeyLocation::Numpad),
            "numpaddecimal" => (Key::Character(".".into()), KeyLocation::Numpad),
            "numpaddivide" => (Key::Character("/".into()), KeyLocation::Numpad),
            "numpadequals" => (Key::Character("=".into()), KeyLocation::Numpad),
            "numpadsubtract" => (Key::Character("-".into()), KeyLocation::Numpad),
            "numpadmultiply" => (Key::Character("*".into()), KeyLocation::Numpad),
            "numpad1" => (Key::Character("1".into()), KeyLocation::Numpad),
            "numpad2" => (Key::Character("2".into()), KeyLocation::Numpad),
            "numpad3" => (Key::Character("3".into()), KeyLocation::Numpad),
            "numpad4" => (Key::Character("4".into()), KeyLocation::Numpad),
            "numpad5" => (Key::Character("5".into()), KeyLocation::Numpad),
            "numpad6" => (Key::Character("6".into()), KeyLocation::Numpad),
            "numpad7" => (Key::Character("7".into()), KeyLocation::Numpad),
            "numpad8" => (Key::Character("8".into()), KeyLocation::Numpad),
            "numpad9" => (Key::Character("9".into()), KeyLocation::Numpad),
            "numpad0" => (Key::Character("0".into()), KeyLocation::Numpad),

            // Special cases
            "tab" => (Key::Named(Tab), KeyLocation::Standard),
            _ => return Err("Unable to find defined 'keycode'".to_string()),
        }
    };

    let trigger = BindingKey::Keycode { key, location };

    let mut res = ModifiersState::empty();
    for modifier in config_key_binding.with.split('|') {
        match modifier.trim().to_lowercase().as_str() {
            "command" | "super" => res.insert(ModifiersState::SUPER),
            "shift" => res.insert(ModifiersState::SHIFT),
            "alt" | "option" => res.insert(ModifiersState::ALT),
            "control" => res.insert(ModifiersState::CONTROL),
            "none" => (),
            _ => (),
        }
    }

    let action_str = config_key_binding.action;
    let mut action: Action = action_str.clone().into();
    // An unknown action parses as `None`, which would silently unbind
    // the default on this key. Reject it so the config error is loud
    // and the default stays.
    if action == Action::None
        && !action_str.is_empty()
        && action_str.to_lowercase() != "none"
    {
        return Err(format!("unknown action '{action_str}'"));
    }
    if !config_key_binding.esc.is_empty() {
        action = Action::Esc(config_key_binding.esc);
    }

    let mut res_mode = ModeWrapper {
        mode: BindingMode::empty(),
        not_mode: BindingMode::empty(),
    };

    for modifier in config_key_binding.mode.split('|') {
        match modifier.trim().to_lowercase().as_str() {
            "appcursor" => res_mode.mode |= BindingMode::APP_CURSOR,
            "~appcursor" => res_mode.not_mode |= BindingMode::APP_CURSOR,
            "appkeypad" => res_mode.mode |= BindingMode::APP_KEYPAD,
            "~appkeypad" => res_mode.not_mode |= BindingMode::APP_KEYPAD,
            "alt" => res_mode.mode |= BindingMode::ALT_SCREEN,
            "~alt" => res_mode.not_mode |= BindingMode::ALT_SCREEN,
            "vi" => res_mode.mode |= BindingMode::VI,
            "~vi" => res_mode.not_mode |= BindingMode::VI,
            _ => {
                res_mode.not_mode |= BindingMode::empty();
                res_mode.mode |= BindingMode::empty();
            }
        }
    }

    Ok(KeyBinding {
        trigger,
        mods: res,
        action,
        mode: res_mode.mode,
        notmode: res_mode.not_mode,
    })
}

/// Legacy (non-kitty) C0 byte for a ctrl+character combo, using the
/// same table and modifier discipline as kitty. The platform is
/// inconsistent about synthesizing these (ctrl+6, ctrl+/), so the byte
/// is computed here instead of trusting the reported text. Returns
/// `None` when the combo has no C0 identity or carries modifiers
/// beyond ctrl (plus alt, which the caller encodes as an ESC prefix,
/// and shift when it only serves to produce the character itself).
///
/// `i`, `m` and `[` are intentionally absent per fixterms: their C0
/// bytes collide with Tab, Enter and Escape, and the platform text
/// already carries them in legacy mode.
pub fn ctrl_seq(key: &Key, text: &str, mods: ModifiersState) -> Option<u8> {
    if !mods.control_key() {
        return None;
    }

    let mut c = {
        let mut it = text.chars();
        match (it.next(), it.next()) {
            (Some(c), None) if c.is_ascii() => c,
            _ => {
                // No single-byte text: fall back to the logical key.
                // Covers layouts whose key produces a non-ASCII char
                // (cyrillic) and platforms reporting no text at all.
                let Key::Character(ch) = key else {
                    return None;
                };
                let mut it = ch.chars();
                let (Some(c), None) = (it.next(), it.next()) else {
                    return None;
                };
                if !c.is_ascii() {
                    return None;
                }
                c
            }
        }
    };

    let mut rest = mods & !(ModifiersState::CONTROL | ModifiersState::ALT);
    // Shift is consumed when it only produced the character itself
    // (ctrl+shift+6 arrives as '^'); for letters it stays, so
    // ctrl+shift+a remains distinguishable from ctrl+a.
    if rest.shift_key() && !c.is_ascii_uppercase() && c != '@' {
        rest &= !ModifiersState::SHIFT;
    }
    if c.is_ascii_uppercase() && !rest.shift_key() {
        // Caps lock without shift.
        c = c.to_ascii_lowercase();
    }
    if !rest.is_empty() {
        return None;
    }

    Some(match c {
        ' ' | '2' | '@' => 0x00,
        '3' => 0x1b,
        '4' | '\\' => 0x1c,
        '5' | ']' => 0x1d,
        '6' | '^' | '~' => 0x1e,
        '7' | '/' | '_' => 0x1f,
        '8' | '?' => 0x7f,
        '0' => b'0',
        '1' => b'1',
        '9' => b'9',
        'i' | 'm' => return None,
        c @ 'a'..='z' => (c as u8) - b'a' + 1,
        _ => return None,
    })
}

pub fn config_key_bindings(
    config_key_bindings: Vec<ConfigKeyBinding>,
    mut bindings: Vec<KeyBinding>,
) -> Vec<KeyBinding> {
    if config_key_bindings.is_empty() {
        return bindings;
    }

    for ckb in config_key_bindings {
        match convert(ckb) {
            Ok(key_binding) => {
                // Remove any default binding that would conflict with this user binding
                // This ensures user bindings always take precedence and prevents conflicts
                bindings.retain(|b| !b.triggers_match(&key_binding));

                tracing::info!("added a new key_binding: {:?}", key_binding);
                bindings.push(key_binding)
            }
            Err(err_message) => {
                tracing::error!("error loading a key binding: {:?}", err_message);
            }
        }
    }

    bindings
}

/// Create hint bindings from configuration
pub fn create_hint_bindings(
    hints_config: &[rio_backend::config::hints::Hint],
) -> Vec<KeyBinding> {
    let mut hint_bindings = Vec::new();

    for hint_config in hints_config {
        if let Some(binding_config) = &hint_config.binding {
            // Parse key using the same logic as in convert()
            let (key, location) = match binding_config.key.to_lowercase().as_str() {
                // Letters
                single_char if single_char.len() == 1 => {
                    (Key::Character(single_char.into()), KeyLocation::Standard)
                }
                // Named keys
                "space" => (Key::Named(Space), KeyLocation::Standard),
                "enter" | "return" => (Key::Named(Enter), KeyLocation::Standard),
                "escape" | "esc" => (Key::Named(Escape), KeyLocation::Standard),
                "tab" => (Key::Named(Tab), KeyLocation::Standard),
                "backspace" => (Key::Named(Backspace), KeyLocation::Standard),
                "delete" => (Key::Named(Delete), KeyLocation::Standard),
                "insert" => (Key::Named(Insert), KeyLocation::Standard),
                "home" => (Key::Named(Home), KeyLocation::Standard),
                "end" => (Key::Named(End), KeyLocation::Standard),
                "pageup" => (Key::Named(PageUp), KeyLocation::Standard),
                "pagedown" => (Key::Named(PageDown), KeyLocation::Standard),
                "up" => (Key::Named(ArrowUp), KeyLocation::Standard),
                "down" => (Key::Named(ArrowDown), KeyLocation::Standard),
                "left" => (Key::Named(ArrowLeft), KeyLocation::Standard),
                "right" => (Key::Named(ArrowRight), KeyLocation::Standard),
                // Function keys
                "f1" => (Key::Named(F1), KeyLocation::Standard),
                "f2" => (Key::Named(F2), KeyLocation::Standard),
                "f3" => (Key::Named(F3), KeyLocation::Standard),
                "f4" => (Key::Named(F4), KeyLocation::Standard),
                "f5" => (Key::Named(F5), KeyLocation::Standard),
                "f6" => (Key::Named(F6), KeyLocation::Standard),
                "f7" => (Key::Named(F7), KeyLocation::Standard),
                "f8" => (Key::Named(F8), KeyLocation::Standard),
                "f9" => (Key::Named(F9), KeyLocation::Standard),
                "f10" => (Key::Named(F10), KeyLocation::Standard),
                "f11" => (Key::Named(F11), KeyLocation::Standard),
                "f12" => (Key::Named(F12), KeyLocation::Standard),
                _ => {
                    tracing::warn!(
                        "Unknown key '{}' in hint binding",
                        binding_config.key
                    );
                    continue;
                }
            };

            // Parse modifiers
            let mut mods = ModifiersState::empty();
            for mod_str in &binding_config.mods {
                match mod_str.to_lowercase().as_str() {
                    "control" | "ctrl" => mods |= ModifiersState::CONTROL,
                    "shift" => mods |= ModifiersState::SHIFT,
                    "alt" | "option" => mods |= ModifiersState::ALT,
                    "super" | "cmd" | "command" => mods |= ModifiersState::SUPER,
                    _ => {
                        tracing::warn!("Unknown modifier '{}' in hint binding", mod_str);
                    }
                }
            }

            let hint_binding = KeyBinding {
                trigger: BindingKey::Keycode { key, location },
                mods,
                mode: BindingMode::empty(),
                notmode: BindingMode::SEARCH | BindingMode::VI,
                action: Action::Hint(std::rc::Rc::new(hint_config.clone())),
            };

            hint_bindings.push(hint_binding);
        }
    }

    hint_bindings
}

/// Automexia's original non-macOS tab scopes.
///
/// - Ctrl+T adds a window-level tab.
/// - Ctrl+Shift+T adds an independent tab to the selected pane/session.
#[cfg(any(test, not(target_os = "macos")))]
fn scoped_tab_key_bindings() -> Vec<KeyBinding> {
    bindings!(
        KeyBinding;
        "t", ModifiersState::CONTROL; Action::TabCreateNew;
        "t", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::LocalTabCreateNew;
    )
}

/// Automexia's classic macOS defaults.
#[cfg(any(test, target_os = "macos"))]
fn automexia_macos_key_bindings(
    use_navigation_key_bindings: bool,
    use_splits: bool,
    config_keyboard: ConfigKeyboard,
) -> Vec<KeyBinding> {
    let mut key_bindings = bindings!(
        KeyBinding;
        "k", ModifiersState::SUPER | ModifiersState::ALT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ClearScreen;
        "0", ModifiersState::SUPER; Action::ResetFontSize;
        "=", ModifiersState::SUPER; Action::IncreaseFontSize;
        "+", ModifiersState::SUPER; Action::IncreaseFontSize;
        "-", ModifiersState::SUPER; Action::DecreaseFontSize;
        Key::Named(Insert), ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH;
            Action::Esc("\x1b[2;2~".into());
        "k", ModifiersState::SUPER, ~BindingMode::VI, ~BindingMode::SEARCH;
            Action::Esc("\x0c".into());
        "k", ModifiersState::SUPER, ~BindingMode::VI; Action::ClearHistory;
        "v", ModifiersState::SUPER, ~BindingMode::VI; Action::Paste;
        "f", ModifiersState::CONTROL | ModifiersState::SUPER; Action::ToggleFullscreen;
        "c", ModifiersState::SUPER; Action::Copy;
        "c", ModifiersState::SUPER, +BindingMode::VI; Action::ClearSelection;
        "a", ModifiersState::SUPER, ~BindingMode::VI, ~BindingMode::SEARCH; Action::SelectAll;
        "q", ModifiersState::SUPER; Action::Quit;
        "n", ModifiersState::SUPER; Action::WindowCreateNew;
        ",", ModifiersState::SUPER; Action::ConfigEditor;
        "p", ModifiersState::SUPER | ModifiersState::SHIFT; Action::OpenCommandPalette;
        "s", ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenSettings;
        "h", ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenConnectionHub;
        "o", ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenActionCenter;
        "m", ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenExtensionMarketplace;
        "l", ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenFontBrowser;
        "t", ModifiersState::SUPER | ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ToggleAppearanceTheme;
        "i", ModifiersState::SUPER | ModifiersState::ALT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::PreviewSelectedImage;
        // Search: local is the familiar Find chord; Shift expands the scope.
        "f", ModifiersState::SUPER, ~BindingMode::VI; Action::SearchForward;
        "f", ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::VI; Action::SearchGlobalForward;
        "b", ModifiersState::SUPER, ~BindingMode::VI; Action::SearchBackward;
        "b", ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::VI; Action::SearchGlobalBackward;
        "c", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchCancel;
        "u", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchClear;
        "w", ModifiersState::CONTROL,  +BindingMode::SEARCH; SearchAction::SearchDeleteWord;
        "p", ModifiersState::CONTROL,  +BindingMode::SEARCH; SearchAction::SearchHistoryPrevious;
        "n", ModifiersState::CONTROL,  +BindingMode::SEARCH; SearchAction::SearchHistoryNext;
        Key::Named(ArrowUp), +BindingMode::SEARCH; SearchAction::SearchHistoryPrevious;
        Key::Named(ArrowDown), +BindingMode::SEARCH; SearchAction::SearchHistoryNext;
        Key::Named(ArrowUp), ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ScrollToPrevPrompt;
        Key::Named(ArrowDown), ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ScrollToNextPrompt;
    );

    if use_navigation_key_bindings {
        key_bindings.extend(bindings!(
            KeyBinding;
            "t", ModifiersState::SUPER; Action::TabCreateNew;
            "t", ModifiersState::SUPER | ModifiersState::SHIFT; Action::LocalTabCreateNew;
            Key::Named(Tab), ModifiersState::CONTROL; Action::SelectNextTab;
            Key::Named(Tab), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::SelectPrevTab;
            "w", ModifiersState::SUPER; Action::CloseCurrentSplitOrTab;
            "w", ModifiersState::SUPER | ModifiersState::SHIFT; Action::TabCloseCurrent;
            "w", ModifiersState::SUPER | ModifiersState::ALT; Action::TabCloseUnfocused;
            "[", ModifiersState::SUPER | ModifiersState::SHIFT; Action::SelectPrevTab;
            "]", ModifiersState::SUPER | ModifiersState::SHIFT; Action::SelectNextTab;
            "[", ModifiersState::SUPER | ModifiersState::ALT; Action::SelectPrevLocalTab;
            "]", ModifiersState::SUPER | ModifiersState::ALT; Action::SelectNextLocalTab;
            "1", ModifiersState::SUPER; Action::SelectTab(0);
            "2", ModifiersState::SUPER; Action::SelectTab(1);
            "3", ModifiersState::SUPER; Action::SelectTab(2);
            "4", ModifiersState::SUPER; Action::SelectTab(3);
            "5", ModifiersState::SUPER; Action::SelectTab(4);
            "6", ModifiersState::SUPER; Action::SelectTab(5);
            "7", ModifiersState::SUPER; Action::SelectTab(6);
            "8", ModifiersState::SUPER; Action::SelectTab(7);
            "9", ModifiersState::SUPER; Action::SelectLastTab;
        ));
    }

    if config_keyboard.disable_ctlseqs_alt {
        key_bindings.extend(bindings!(
            KeyBinding;
            Key::Named(ArrowLeft), ModifiersState::ALT, ~BindingMode::VI;
                Action::Esc("\x1bb".into());
            Key::Named(ArrowRight), ModifiersState::ALT, ~BindingMode::VI;
                Action::Esc("\x1bf".into());
        ));
    }

    if use_splits {
        key_bindings.extend(bindings!(
            KeyBinding;
            "d", ModifiersState::SUPER, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitRight;
            "d", ModifiersState::SUPER | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitDown;
            "r", ModifiersState::SUPER | ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::CloneSplitRight;
            "d", ModifiersState::SUPER | ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::CloneSplitDown;
            "]", ModifiersState::SUPER, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SelectNextSplit;
            "[", ModifiersState::SUPER, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SelectPrevSplit;
            Key::Named(ArrowLeft), ModifiersState::SUPER | ModifiersState::ALT; Action::SelectPaneLeft;
            Key::Named(ArrowRight), ModifiersState::SUPER | ModifiersState::ALT; Action::SelectPaneRight;
            Key::Named(ArrowUp), ModifiersState::SUPER | ModifiersState::ALT; Action::SelectPaneUp;
            Key::Named(ArrowDown), ModifiersState::SUPER | ModifiersState::ALT; Action::SelectPaneDown;
            Key::Named(ArrowUp), ModifiersState::CONTROL | ModifiersState::SUPER, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerUp;
            Key::Named(ArrowDown), ModifiersState::CONTROL | ModifiersState::SUPER, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerDown;
            Key::Named(ArrowLeft), ModifiersState::CONTROL | ModifiersState::SUPER, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerLeft;
            Key::Named(ArrowRight), ModifiersState::CONTROL | ModifiersState::SUPER, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerRight;
        ));
    }

    key_bindings
}

/// Automexia's classic Windows defaults.
#[cfg(any(test, target_os = "windows"))]
fn automexia_windows_key_bindings(
    use_navigation_key_bindings: bool,
    use_splits: bool,
) -> Vec<KeyBinding> {
    let mut key_bindings = bindings!(
        KeyBinding;
        "q", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::Quit;
        "k", ModifiersState::CONTROL | ModifiersState::ALT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ClearScreen;
        "b", ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::VI; Action::SearchBackward;
        ",", ModifiersState::CONTROL; Action::ConfigEditor;
        Key::Named(Insert), ModifiersState::SHIFT, ~BindingMode::VI; Action::PasteSelection;
        "c", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::Copy;
        "c", ModifiersState::CONTROL | ModifiersState::SHIFT, +BindingMode::VI; Action::ClearSelection;
        "v", ModifiersState::CONTROL, ~BindingMode::VI; Action::Paste;
        "v", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::Paste;
        "0", ModifiersState::CONTROL; Action::ResetFontSize;
        "=", ModifiersState::CONTROL; Action::IncreaseFontSize;
        "+", ModifiersState::CONTROL; Action::IncreaseFontSize;
        "-", ModifiersState::CONTROL; Action::DecreaseFontSize;
        "n", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::WindowCreateNew;
        "p", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::OpenCommandPalette;
        "s", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenSettings;
        "h", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenConnectionHub;
        "o", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenActionCenter;
        "m", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenExtensionMarketplace;
        "l", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenFontBrowser;
        "i", ModifiersState::CONTROL | ModifiersState::ALT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::PreviewSelectedImage;
        "a", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH; Action::SelectAll;
        // Search: Ctrl+F is pane-local; Shift expands the scope.
        "f", ModifiersState::CONTROL, ~BindingMode::VI; Action::SearchForward;
        "f", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::SearchGlobalForward;
        "b", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::SearchGlobalBackward;
        "c", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchCancel;
        "u", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchClear;
        "w", ModifiersState::CONTROL,  +BindingMode::SEARCH; SearchAction::SearchDeleteWord;
        "p", ModifiersState::CONTROL,  +BindingMode::SEARCH; SearchAction::SearchHistoryPrevious;
        "n", ModifiersState::CONTROL,  +BindingMode::SEARCH; SearchAction::SearchHistoryNext;
        Key::Named(ArrowUp), +BindingMode::SEARCH; SearchAction::SearchHistoryPrevious;
        Key::Named(ArrowDown), +BindingMode::SEARCH; SearchAction::SearchHistoryNext;
        Key::Named(ArrowUp), ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ScrollToPrevPrompt;
        Key::Named(ArrowDown), ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ScrollToNextPrompt;

        Key::Named(Space), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::ToggleViMode;
        Key::Named(Space), ModifiersState::CONTROL | ModifiersState::ALT; Action::ToggleQuake;
        "t", ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ToggleAppearanceTheme;
        "k", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::ClearHistory;
        Key::Named(F11); Action::ToggleFullscreen;
        Key::Named(Enter), ModifiersState::ALT; Action::ToggleFullscreen;
        Key::Named(Backspace), ModifiersState::CONTROL, ~BindingMode::VI; Action::Esc("\u{0017}".into());
    );

    key_bindings.extend(scoped_tab_key_bindings());

    if use_navigation_key_bindings {
        key_bindings.extend(bindings!(
            KeyBinding;
            Key::Named(Tab), ModifiersState::CONTROL; Action::SelectNextTab;
            Key::Named(Tab), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::SelectPrevTab;
            Key::Named(PageUp), ModifiersState::CONTROL; Action::SelectPrevTab;
            Key::Named(PageDown), ModifiersState::CONTROL; Action::SelectNextTab;
            Key::Named(PageUp), ModifiersState::ALT; Action::SelectPrevLocalTab;
            Key::Named(PageDown), ModifiersState::ALT; Action::SelectNextLocalTab;
            Key::Named(PageUp), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::MoveCurrentTabToPrev;
            Key::Named(PageDown), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::MoveCurrentTabToNext;
            "w", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::CloseCurrentSplitOrTab;
            Key::Named(F4), ModifiersState::CONTROL; Action::TabCloseCurrent;
            Key::Named(F4), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::TabCloseUnfocused;
            "1", ModifiersState::CONTROL; Action::SelectTab(0);
            "2", ModifiersState::CONTROL; Action::SelectTab(1);
            "3", ModifiersState::CONTROL; Action::SelectTab(2);
            "4", ModifiersState::CONTROL; Action::SelectTab(3);
            "5", ModifiersState::CONTROL; Action::SelectTab(4);
            "6", ModifiersState::CONTROL; Action::SelectTab(5);
            "7", ModifiersState::CONTROL; Action::SelectTab(6);
            "8", ModifiersState::CONTROL; Action::SelectTab(7);
            "9", ModifiersState::CONTROL; Action::SelectLastTab;
        ));
    }

    if use_splits {
        key_bindings.extend(bindings!(
            KeyBinding;
            // R/D gives direction; Shift starts fresh instead of cloning.
            "r", ModifiersState::ALT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::CloneSplitRight;
            "d", ModifiersState::ALT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::CloneSplitDown;
            "r", ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitRight;
            "d", ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitDown;
            Key::Named(F6), ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SelectNextSplit;
            Key::Named(F6), ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SelectPrevSplit;
            Key::Named(ArrowLeft), ModifiersState::ALT; Action::SelectPaneLeft;
            Key::Named(ArrowRight), ModifiersState::ALT; Action::SelectPaneRight;
            Key::Named(ArrowUp), ModifiersState::ALT; Action::SelectPaneUp;
            Key::Named(ArrowDown), ModifiersState::ALT; Action::SelectPaneDown;
            Key::Named(ArrowUp), ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerUp;
            Key::Named(ArrowDown), ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerDown;
            Key::Named(ArrowLeft), ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerLeft;
            Key::Named(ArrowRight), ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerRight;
        ));
    }

    key_bindings
}

/// Automexia's classic Linux/BSD defaults. Pane focus uses F6/Shift+F6 so the
/// historical Ctrl+Shift+bracket tab shortcuts never have a duplicate owner.
#[cfg(any(test, not(any(target_os = "macos", target_os = "windows"))))]
fn automexia_unix_key_bindings(
    use_navigation_key_bindings: bool,
    use_splits: bool,
) -> Vec<KeyBinding> {
    let mut key_bindings = bindings!(
        KeyBinding;
        "q", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::Quit;
        "k", ModifiersState::CONTROL | ModifiersState::ALT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ClearScreen;
        "b", ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::VI; Action::SearchBackward;
        Key::Named(F11); Action::ToggleFullscreen;
        "v", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::Paste;
        "c", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::Copy;
        "c", ModifiersState::CONTROL | ModifiersState::SHIFT, +BindingMode::VI; Action::ClearSelection;
        Key::Named(Insert), ModifiersState::SHIFT, ~BindingMode::VI; Action::PasteSelection;
        "0", ModifiersState::CONTROL; Action::ResetFontSize;
        "=", ModifiersState::CONTROL; Action::IncreaseFontSize;
        "+", ModifiersState::CONTROL; Action::IncreaseFontSize;
        "-", ModifiersState::CONTROL; Action::DecreaseFontSize;
        "n", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::WindowCreateNew;
        ",", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::ConfigEditor;
        "p", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::OpenCommandPalette;
        "s", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenSettings;
        "h", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenConnectionHub;
        "o", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenActionCenter;
        "m", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenExtensionMarketplace;
        "l", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::OpenFontBrowser;
        "t", ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ToggleAppearanceTheme;
        "i", ModifiersState::CONTROL | ModifiersState::ALT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::PreviewSelectedImage;

        // Search: Ctrl+F is pane-local; Shift expands the scope.
        "f", ModifiersState::CONTROL, ~BindingMode::VI; Action::SearchForward;
        "f", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::SearchGlobalForward;
        "b", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::SearchGlobalBackward;
        "c", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchCancel;
        "u", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchClear;
        "w", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchDeleteWord;
        "p", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchHistoryPrevious;
        "n", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchHistoryNext;
        Key::Named(ArrowUp), +BindingMode::SEARCH; SearchAction::SearchHistoryPrevious;
        Key::Named(ArrowDown), +BindingMode::SEARCH; SearchAction::SearchHistoryNext;
        Key::Named(ArrowUp), ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ScrollToPrevPrompt;
        Key::Named(ArrowDown), ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::ScrollToNextPrompt;
    );

    key_bindings.extend(scoped_tab_key_bindings());

    if use_navigation_key_bindings {
        key_bindings.extend(bindings!(
            KeyBinding;
            Key::Named(Tab), ModifiersState::CONTROL; Action::SelectNextTab;
            Key::Named(Tab), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::SelectPrevTab;
            "[", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::SelectPrevTab;
            "]", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::SelectNextTab;
            Key::Named(PageUp), ModifiersState::ALT; Action::SelectPrevLocalTab;
            Key::Named(PageDown), ModifiersState::ALT; Action::SelectNextLocalTab;
            "w", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::CloseCurrentSplitOrTab;
            Key::Named(F4), ModifiersState::CONTROL; Action::TabCloseCurrent;
            Key::Named(F4), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::TabCloseUnfocused;
        ));
    }

    if use_splits {
        key_bindings.extend(bindings!(
            KeyBinding;
            // R/D gives direction; Shift starts fresh instead of cloning.
            "r", ModifiersState::ALT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::CloneSplitRight;
            "d", ModifiersState::ALT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::CloneSplitDown;
            "r", ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitRight;
            "d", ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitDown;
            Key::Named(F6), ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SelectNextSplit;
            Key::Named(F6), ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SelectPrevSplit;
            Key::Named(ArrowLeft), ModifiersState::ALT; Action::SelectPaneLeft;
            Key::Named(ArrowRight), ModifiersState::ALT; Action::SelectPaneRight;
            Key::Named(ArrowUp), ModifiersState::ALT; Action::SelectPaneUp;
            Key::Named(ArrowDown), ModifiersState::ALT; Action::SelectPaneDown;
            Key::Named(ArrowUp), ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerUp;
            Key::Named(ArrowDown), ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerDown;
            Key::Named(ArrowLeft), ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerLeft;
            Key::Named(ArrowRight), ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerRight;
        ));
    }

    key_bindings
}

#[cfg(target_os = "windows")]
pub fn platform_key_bindings(
    use_navigation_key_bindings: bool,
    use_splits: bool,
    _: ConfigKeyboard,
) -> Vec<KeyBinding> {
    automexia_windows_key_bindings(use_navigation_key_bindings, use_splits)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn platform_key_bindings(
    use_navigation_key_bindings: bool,
    use_splits: bool,
    _: ConfigKeyboard,
) -> Vec<KeyBinding> {
    automexia_unix_key_bindings(use_navigation_key_bindings, use_splits)
}

#[cfg(target_os = "macos")]
pub fn platform_key_bindings(
    use_navigation_key_bindings: bool,
    use_splits: bool,
    config_keyboard: ConfigKeyboard,
) -> Vec<KeyBinding> {
    let mut key_bindings = automexia_macos_key_bindings(
        use_navigation_key_bindings,
        use_splits,
        config_keyboard,
    );
    key_bindings.extend(bindings!(
        KeyBinding;
        "h", ModifiersState::SUPER; Action::Hide;
        "h", ModifiersState::SUPER | ModifiersState::ALT; Action::HideOtherApplications;
        "m", ModifiersState::SUPER; Action::Minimize;
    ));
    key_bindings
}

#[cfg(test)]
mod tests {
    use super::*;

    use rio_window::keyboard::ModifiersState;

    type MockBinding = Binding<usize>;

    impl Default for MockBinding {
        fn default() -> Self {
            Self {
                mods: Default::default(),
                action: Action::None,
                mode: BindingMode::empty(),
                notmode: BindingMode::empty(),
                trigger: Default::default(),
            }
        }
    }

    #[test]
    fn binding_matches_itself() {
        let binding = MockBinding::default();
        let identical_binding = MockBinding::default();

        assert!(binding.triggers_match(&identical_binding));
        assert!(identical_binding.triggers_match(&binding));
    }

    #[test]
    fn binding_matches_different_action() {
        let binding = MockBinding::default();
        let different_action = MockBinding {
            action: Action::ClearHistory,
            ..MockBinding::default()
        };

        assert!(binding.triggers_match(&different_action));
        assert!(different_action.triggers_match(&binding));
    }

    #[test]
    fn default_mouse_clipboard_bindings_preserve_primary_selection_ownership() {
        let bindings = default_mouse_bindings();
        let right_paste = bindings.iter().find(|binding| {
            binding.trigger == MouseButton::Right
                && binding.mods.is_empty()
                && binding.action == Action::Paste
        });
        assert_eq!(
            right_paste.map(|binding| binding.notmode.clone()),
            Some(BindingMode::VI)
        );
        assert!(bindings.iter().any(|binding| {
            binding.trigger == MouseButton::Middle
                && binding.action == Action::PasteSelection
                && binding.notmode == BindingMode::VI
        }));
        assert!(!bindings.iter().any(|binding| {
            binding.trigger == MouseButton::Left
                && matches!(binding.action, Action::Paste | Action::PasteSelection)
        }));
    }

    #[test]
    fn mods_binding_requires_strict_match() {
        let superset_mods = MockBinding {
            mods: ModifiersState::all(),
            ..MockBinding::default()
        };
        let subset_mods = MockBinding {
            mods: ModifiersState::ALT,
            ..MockBinding::default()
        };

        assert!(!superset_mods.triggers_match(&subset_mods));
        assert!(!subset_mods.triggers_match(&superset_mods));
    }

    #[test]
    fn binding_matches_identical_mode() {
        let b1 = MockBinding {
            mode: BindingMode::ALT_SCREEN,
            ..MockBinding::default()
        };
        let b2 = MockBinding {
            mode: BindingMode::ALT_SCREEN,
            ..MockBinding::default()
        };

        assert!(b1.triggers_match(&b2));
        assert!(b2.triggers_match(&b1));
    }

    #[test]
    fn binding_without_mode_matches_any_mode() {
        let b1 = MockBinding::default();
        let b2 = MockBinding {
            mode: BindingMode::APP_KEYPAD,
            notmode: BindingMode::ALT_SCREEN,
            ..MockBinding::default()
        };

        assert!(b1.triggers_match(&b2));
    }

    #[test]
    fn binding_with_mode_matches_empty_mode() {
        let b1 = MockBinding {
            mode: BindingMode::APP_KEYPAD,
            notmode: BindingMode::ALT_SCREEN,
            ..MockBinding::default()
        };
        let b2 = MockBinding::default();

        assert!(b1.triggers_match(&b2));
        assert!(b2.triggers_match(&b1));
    }

    #[test]
    fn binding_matches_modes() {
        let b1 = MockBinding {
            mode: BindingMode::ALT_SCREEN | BindingMode::APP_KEYPAD,
            ..MockBinding::default()
        };
        let b2 = MockBinding {
            mode: BindingMode::APP_KEYPAD,
            ..MockBinding::default()
        };

        assert!(b1.triggers_match(&b2));
        assert!(b2.triggers_match(&b1));
    }

    #[test]
    fn binding_matches_partial_intersection() {
        let b1 = MockBinding {
            mode: BindingMode::ALT_SCREEN | BindingMode::APP_KEYPAD,
            ..MockBinding::default()
        };
        let b2 = MockBinding {
            mode: BindingMode::APP_KEYPAD | BindingMode::APP_CURSOR,
            ..MockBinding::default()
        };

        assert!(b1.triggers_match(&b2));
        assert!(b2.triggers_match(&b1));
    }

    #[test]
    fn binding_mismatches_notmode() {
        let b1 = MockBinding {
            mode: BindingMode::ALT_SCREEN,
            ..MockBinding::default()
        };
        let b2 = MockBinding {
            notmode: BindingMode::ALT_SCREEN,
            ..MockBinding::default()
        };

        assert!(!b1.triggers_match(&b2));
        assert!(!b2.triggers_match(&b1));
    }

    #[test]
    fn binding_mismatches_unrelated() {
        let b1 = MockBinding {
            mode: BindingMode::ALT_SCREEN,
            ..MockBinding::default()
        };
        let b2 = MockBinding {
            mode: BindingMode::APP_KEYPAD,
            ..MockBinding::default()
        };

        assert!(!b1.triggers_match(&b2));
        assert!(!b2.triggers_match(&b1));
    }

    #[test]
    fn binding_matches_notmodes() {
        let subset_notmodes = MockBinding {
            notmode: BindingMode::VI | BindingMode::APP_CURSOR,
            ..MockBinding::default()
        };
        let superset_notmodes = MockBinding {
            notmode: BindingMode::APP_CURSOR,
            ..MockBinding::default()
        };

        assert!(subset_notmodes.triggers_match(&superset_notmodes));
        assert!(superset_notmodes.triggers_match(&subset_notmodes));
    }

    #[test]
    fn binding_matches_mode_notmode() {
        let b1 = MockBinding {
            mode: BindingMode::VI,
            notmode: BindingMode::APP_CURSOR,
            ..MockBinding::default()
        };
        let b2 = MockBinding {
            notmode: BindingMode::APP_CURSOR,
            ..MockBinding::default()
        };

        assert!(b1.triggers_match(&b2));
        assert!(b2.triggers_match(&b1));
    }

    // #[test]
    // fn binding_trigger_input() {
    //     let binding = MockBinding { trigger: 13, ..MockBinding::default() };

    //     let mods = binding.mods;
    //     let mode = binding.mode;

    //     assert!(binding.is_triggered_by(mode, mods, &13));
    //     assert!(!binding.is_triggered_by(mode, mods, &32));
    // }

    // #[test]
    // fn binding_trigger_mods() {
    //     let binding = MockBinding {
    //         mods: ModifiersState::ALT | ModifiersState::SUPER,
    //         ..MockBinding::default()
    //     };

    //     let superset_mods = ModifiersState::all();
    //     let subset_mods = ModifiersState::empty();

    //     let t = binding.trigger;
    //     let mode = binding.mode;

    //     assert!(binding.is_triggered_by(mode, binding.mods, &t));
    //     assert!(!binding.is_triggered_by(mode, superset_mods, &t));
    //     assert!(!binding.is_triggered_by(mode, subset_mods, &t));
    // }

    #[test]
    fn binding_trigger_modes() {
        let binding = MockBinding {
            mode: BindingMode::ALT_SCREEN,
            ..MockBinding::default()
        };

        let t = binding.trigger;
        let mods = binding.mods;

        assert!(!binding.is_triggered_by(BindingMode::VI, mods, &t));
        assert!(binding.is_triggered_by(BindingMode::ALT_SCREEN, mods, &t));
        assert!(binding.is_triggered_by(
            BindingMode::ALT_SCREEN | BindingMode::VI,
            mods,
            &t
        ));
    }

    #[test]
    fn binding_trigger_notmodes() {
        let binding = MockBinding {
            notmode: BindingMode::ALT_SCREEN,
            ..MockBinding::default()
        };

        let t = binding.trigger;
        let mods = binding.mods;

        assert!(binding.is_triggered_by(BindingMode::VI, mods, &t));
        assert!(!binding.is_triggered_by(BindingMode::ALT_SCREEN, mods, &t));
        assert!(!binding.is_triggered_by(
            BindingMode::ALT_SCREEN | BindingMode::VI,
            mods,
            &t
        ));
    }

    #[test]
    fn settings_defaults_are_distinct_from_config_editor_and_leave_application_modes() {
        use automexia_keybindings::PlatformFamily::{LinuxBsd, Macos, Windows};
        for (platform, modifiers, editor_modifiers) in [
            (Windows, ModifiersState::CONTROL, ModifiersState::CONTROL),
            (
                LinuxBsd,
                ModifiersState::CONTROL,
                ModifiersState::CONTROL | ModifiersState::SHIFT,
            ),
            (Macos, ModifiersState::SUPER, ModifiersState::SUPER),
        ] {
            let bindings =
                test_platform_defaults(&rio_backend::config::Config::default(), platform);
            let settings: Vec<_> = bindings
                .iter()
                .filter(|binding| binding.action == Action::OpenSettings)
                .collect();
            assert_eq!(settings.len(), 1, "{platform:?}");
            let binding = settings[0];
            assert_eq!(binding.mods, modifiers | ModifiersState::SHIFT);
            assert!(matches!(&binding.trigger, BindingKey::Keycode {
                key: Key::Character(value), ..
            } if value == "s"));
            assert!(binding.is_triggered_by(
                BindingMode::empty(),
                binding.mods,
                &binding.trigger
            ));
            for mode in [
                BindingMode::ALT_SCREEN,
                BindingMode::SEARCH,
                BindingMode::VI,
            ] {
                assert!(!binding.is_triggered_by(mode, binding.mods, &binding.trigger));
            }
            assert!(bindings.iter().any(|binding| binding.action
                == Action::ConfigEditor
                && binding.mods == editor_modifiers
                && matches!(&binding.trigger, BindingKey::Keycode {
                    key: Key::Character(value), ..
                } if value == ",")));
            assert_eq!(
                bindings
                    .iter()
                    .filter(|candidate| {
                        candidate.is_triggered_by(
                            BindingMode::empty(),
                            binding.mods,
                            &binding.trigger,
                        )
                    })
                    .count(),
                1,
                "Settings must not share a trigger with another default"
            );
        }
    }

    #[test]
    fn bindings_overwrite() {
        let bindings = bindings!(
            KeyBinding;
            "q", ModifiersState::SUPER; Action::Quit;
            ",", ModifiersState::SUPER; Action::ConfigEditor;
        );

        let config_bindings = vec![ConfigKeyBinding {
            key: String::from("q"),
            action: String::from("receivechar"),
            with: String::from("super"),
            esc: String::from(""),
            mode: String::from(""),
        }];

        let new_bindings = config_key_bindings(config_bindings, bindings);

        assert_eq!(new_bindings.len(), 2);
        assert_eq!(new_bindings[1].action, Action::ReceiveChar);
    }

    #[test]
    fn bindings_conflict_resolution() {
        // Test that conflicting bindings are properly replaced
        let bindings = bindings!(
            KeyBinding;
            Key::Named(PageUp), ModifiersState::empty(); Action::Esc("\x1b[5~".into());
            Key::Named(PageDown), ModifiersState::empty(); Action::Esc("\x1b[6~".into());
        );

        // User wants to use PageUp/PageDown for scrolling
        let config_bindings = vec![
            ConfigKeyBinding {
                key: String::from("pageup"),
                action: String::from("scroll(1)"),
                with: String::from(""),
                esc: String::from(""),
                mode: String::from(""),
            },
            ConfigKeyBinding {
                key: String::from("pagedown"),
                action: String::from("scroll(-1)"),
                with: String::from(""),
                esc: String::from(""),
                mode: String::from(""),
            },
        ];

        let new_bindings = config_key_bindings(config_bindings, bindings);

        // Should have 2 bindings (the original defaults should be replaced)
        assert_eq!(new_bindings.len(), 2);

        // Check that the actions were updated to scroll actions
        let has_scroll_actions = new_bindings
            .iter()
            .any(|b| matches!(b.action, Action::Scroll(_)));
        assert!(has_scroll_actions);
    }

    #[test]
    fn ctrl_seq_map() {
        let ctrl = ModifiersState::CONTROL;
        let shift = ModifiersState::SHIFT;
        let alt = ModifiersState::ALT;
        let key = |s: &str| Key::Character(s.into());

        // macOS shape: the platform reports the plain char as text.
        assert_eq!(ctrl_seq(&key("6"), "6", ctrl), Some(0x1e));
        assert_eq!(ctrl_seq(&key("/"), "/", ctrl), Some(0x1f));
        // Windows shape: no text at all.
        assert_eq!(ctrl_seq(&key("6"), "", ctrl), Some(0x1e));
        // ctrl+shift+6 arrives as '^': shift is consumed.
        assert_eq!(ctrl_seq(&key("^"), "^", ctrl | shift), Some(0x1e));
        // Alt passes through; the caller adds the ESC prefix.
        assert_eq!(ctrl_seq(&key("6"), "6", ctrl | alt), Some(0x1e));
        // Letters map, except the fixterms exclusions.
        assert_eq!(ctrl_seq(&key("q"), "q", ctrl), Some(0x11));
        assert_eq!(ctrl_seq(&key("r"), "r", ctrl), Some(0x12));
        assert_eq!(ctrl_seq(&key("i"), "i", ctrl), None);
        assert_eq!(ctrl_seq(&key("m"), "m", ctrl), None);
        // ctrl+shift+letter stays distinguishable: no C0 collapse.
        assert_eq!(ctrl_seq(&key("A"), "A", ctrl | shift), None);
        // Digit passthrough per kitty.
        assert_eq!(ctrl_seq(&key("1"), "1", ctrl), Some(b'1'));
        // Super combos never collapse to C0.
        assert_eq!(ctrl_seq(&key("6"), "6", ctrl | ModifiersState::SUPER), None);
        assert_eq!(ctrl_seq(&key("6"), "6", ModifiersState::empty()), None);
        assert_eq!(ctrl_seq(&Key::Named(Enter), "", ctrl), None);
    }

    #[test]
    fn unknown_action_is_rejected_and_default_survives() {
        let bindings = bindings!(
            KeyBinding;
            "q", ModifiersState::SUPER; Action::Quit;
        );
        let config_bindings = vec![ConfigKeyBinding {
            key: String::from("q"),
            action: String::from("quitt"),
            with: String::from("super"),
            esc: String::from(""),
            mode: String::from(""),
        }];
        let new_bindings = config_key_bindings(config_bindings, bindings);
        assert_eq!(new_bindings.len(), 1);
        assert_eq!(new_bindings[0].action, Action::Quit);
    }

    #[test]
    fn clone_actions_parse_with_stable_configuration_names() {
        assert_eq!(
            Action::from("clonesplitright".to_string()),
            Action::CloneSplitRight
        );
        assert_eq!(
            Action::from("CloneSplitDown".to_string()),
            Action::CloneSplitDown
        );
        assert_eq!(Action::from("splitright".to_string()), Action::SplitRight);
        assert_eq!(Action::from("splitdown".to_string()), Action::SplitDown);
        assert_eq!(
            Action::from("reloadconfig".to_string()),
            Action::ReloadConfig
        );
        assert_eq!(Action::from("closewindow".to_string()), Action::WindowClose);
        assert_eq!(Action::from("clearscreen".to_string()), Action::ClearScreen);
        assert_eq!(
            Action::from("PreviewSelectedImage".to_string()),
            Action::PreviewSelectedImage
        );
        assert_eq!(
            Action::from("SelectPaneLeft".to_string()),
            Action::SelectPaneLeft
        );
        assert_eq!(
            Action::from("SelectNextLocalTab".to_string()),
            Action::SelectNextLocalTab
        );
        assert_eq!(
            Action::from("SearchGlobalForward".to_string()),
            Action::SearchGlobalForward
        );
        assert_eq!(
            Action::from("SearchGlobalBackward".to_string()),
            Action::SearchGlobalBackward
        );
    }

    #[test]
    fn local_and_global_search_defaults_are_memorable_and_scope_safe() {
        let trigger = BindingKey::Keycode {
            key: Key::Character("f".into()),
            location: KeyLocation::Standard,
        };
        let assert_search_pair = |bindings: &[KeyBinding], local_mods, global_mods| {
            let local = bindings
                .iter()
                .find(|binding| binding.trigger == trigger && binding.mods == local_mods);
            let global = bindings.iter().find(|binding| {
                binding.trigger == trigger && binding.mods == global_mods
            });
            assert_eq!(
                local.map(|binding| &binding.action),
                Some(&Action::SearchForward)
            );
            assert_eq!(
                global.map(|binding| &binding.action),
                Some(&Action::SearchGlobalForward)
            );
            for binding in [local.unwrap(), global.unwrap()] {
                assert!(
                    !binding.notmode.contains(BindingMode::SEARCH),
                    "search shortcuts must remain active so they can switch scope in place"
                );
                assert!(binding.notmode.contains(BindingMode::VI));
            }
        };

        assert_search_pair(
            &automexia_windows_key_bindings(true, true),
            ModifiersState::CONTROL,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        assert_search_pair(
            &automexia_unix_key_bindings(true, true),
            ModifiersState::CONTROL,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        );
        assert_search_pair(
            &automexia_macos_key_bindings(true, true, ConfigKeyboard::default()),
            ModifiersState::SUPER,
            ModifiersState::SUPER | ModifiersState::SHIFT,
        );
    }

    #[test]
    fn feature_surface_actions_parse_with_stable_configuration_names() {
        for (name, action) in [
            ("OpenConnectionHub", Action::OpenConnectionHub),
            ("OpenActionCenter", Action::OpenActionCenter),
            ("OpenExtensionMarketplace", Action::OpenExtensionMarketplace),
            ("OpenFontBrowser", Action::OpenFontBrowser),
        ] {
            assert_eq!(Action::from(name.to_string()), action, "{name}");
        }
    }

    #[test]
    fn selection_actions_parse_with_stable_configuration_names() {
        for (name, motion) in [
            ("ExtendSelectionLeft", SelectionMotion::Left),
            ("ExtendSelectionRight", SelectionMotion::Right),
            ("ExtendSelectionUp", SelectionMotion::Up),
            ("ExtendSelectionDown", SelectionMotion::Down),
            ("ExtendSelectionWordLeft", SelectionMotion::WordLeft),
            ("ExtendSelectionWordRight", SelectionMotion::WordRight),
        ] {
            assert_eq!(
                Action::from(name.to_string()),
                Action::ExtendSelection(motion)
            );
        }
    }

    #[test]
    fn keyboard_selection_defaults_are_local_collision_free_and_mode_safe() {
        let config = rio_backend::config::Config::default();
        let bindings = default_key_bindings(&config);
        let expected = [
            (ArrowLeft, ModifiersState::SHIFT, SelectionMotion::Left),
            (ArrowRight, ModifiersState::SHIFT, SelectionMotion::Right),
            (ArrowUp, ModifiersState::SHIFT, SelectionMotion::Up),
            (ArrowDown, ModifiersState::SHIFT, SelectionMotion::Down),
            (
                ArrowLeft,
                ModifiersState::CONTROL | ModifiersState::SHIFT,
                SelectionMotion::WordLeft,
            ),
            (
                ArrowRight,
                ModifiersState::CONTROL | ModifiersState::SHIFT,
                SelectionMotion::WordRight,
            ),
        ];

        let selection_bindings: Vec<_> = bindings
            .iter()
            .filter(|binding| matches!(binding.action, Action::ExtendSelection(_)))
            .cloned()
            .collect();
        assert_eq!(selection_bindings.len(), expected.len());
        assert_no_overlapping_shortcuts("keyboard selection", &selection_bindings);
        let other_bindings: Vec<_> = bindings
            .iter()
            .filter(|binding| !matches!(binding.action, Action::ExtendSelection(_)))
            .cloned()
            .collect();
        assert_no_cross_table_overlaps(
            "keyboard selection versus common defaults",
            &selection_bindings,
            &other_bindings,
        );

        for (key, modifiers, motion) in expected {
            let trigger = BindingKey::Keycode {
                key: Key::Named(key),
                location: KeyLocation::Standard,
            };
            let matching: Vec<_> = selection_bindings
                .iter()
                .filter(|binding| binding.trigger == trigger && binding.mods == modifiers)
                .collect();
            assert_eq!(matching.len(), 1, "missing or duplicate {motion:?}");
            let binding = matching[0];
            assert_eq!(binding.action, Action::ExtendSelection(motion));
            assert!(binding.notmode.contains(BindingMode::VI));
            assert!(binding.notmode.contains(BindingMode::SEARCH));
            assert!(binding.is_triggered_by(BindingMode::empty(), modifiers, &trigger));
            assert!(!binding.is_triggered_by(BindingMode::VI, modifiers, &trigger));
            assert!(!binding.is_triggered_by(BindingMode::SEARCH, modifiers, &trigger));
        }
    }

    #[test]
    fn user_binding_can_override_shift_left_selection() {
        let config = rio_backend::config::Config::default();
        let updated = config_key_bindings(
            vec![ConfigKeyBinding {
                key: "left".to_string(),
                action: "receivechar".to_string(),
                with: "shift".to_string(),
                esc: String::new(),
                mode: String::new(),
            }],
            default_key_bindings(&config),
        );
        let trigger = BindingKey::Keycode {
            key: Key::Named(ArrowLeft),
            location: KeyLocation::Standard,
        };
        let matching: Vec<_> = updated
            .iter()
            .filter(|binding| {
                binding.trigger == trigger && binding.mods == ModifiersState::SHIFT
            })
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].action, Action::ReceiveChar);
    }

    #[test]
    fn automexia_tab_scopes_use_the_original_shortcuts() {
        assert_eq!(
            Action::from("CreateLocalTab".to_string()),
            Action::LocalTabCreateNew
        );
        let bindings = scoped_tab_key_bindings();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].mods, ModifiersState::CONTROL);
        assert_eq!(bindings[0].action, Action::TabCreateNew);
        assert_eq!(
            bindings[1].mods,
            ModifiersState::CONTROL | ModifiersState::SHIFT
        );
        assert_eq!(bindings[1].action, Action::LocalTabCreateNew);
        assert_eq!(bindings[0].trigger, bindings[1].trigger);
        assert_ne!(bindings[0].mods, bindings[1].mods);
    }

    #[test]
    fn user_binding_can_override_ctrl_t_without_changing_local_tab_shortcut() {
        let updated = config_key_bindings(
            vec![ConfigKeyBinding {
                key: "t".to_string(),
                action: "receivechar".to_string(),
                with: "control".to_string(),
                esc: String::new(),
                mode: String::new(),
            }],
            scoped_tab_key_bindings(),
        );
        assert!(updated.iter().any(|binding| {
            binding.mods == ModifiersState::CONTROL
                && binding.action == Action::ReceiveChar
        }));
        assert!(updated.iter().any(|binding| {
            binding.mods == ModifiersState::CONTROL | ModifiersState::SHIFT
                && binding.action == Action::LocalTabCreateNew
        }));
        assert!(!updated.iter().any(|binding| {
            binding.mods == ModifiersState::CONTROL
                && binding.action == Action::TabCreateNew
        }));
    }

    #[test]
    fn platform_defaults_preserve_shell_history_and_eof_input() {
        for use_splits in [false, true] {
            for (platform, bindings) in [
                ("Windows", automexia_windows_key_bindings(true, use_splits)),
                ("Unix", automexia_unix_key_bindings(true, use_splits)),
                (
                    "macOS",
                    automexia_macos_key_bindings(
                        true,
                        use_splits,
                        ConfigKeyboard::default(),
                    ),
                ),
            ] {
                // Exercise complete platform tables: a helper-only test missed
                // the fallback owner that consumed Readline input in real panes.
                for (text, expected) in [("r", 0x12), ("d", 0x04)] {
                    let key = Key::Character(text.into());
                    let trigger = BindingKey::Keycode {
                        key: key.clone(),
                        location: KeyLocation::Standard,
                    };
                    for mode in [BindingMode::empty(), BindingMode::ALT_SCREEN] {
                        assert!(
                            !bindings.iter().any(|binding| binding.is_triggered_by(
                                mode.clone(),
                                ModifiersState::CONTROL,
                                &trigger,
                            )),
                            "{platform} consumed shell Ctrl+{text} in {mode:?}"
                        );
                    }
                    assert_eq!(
                        ctrl_seq(&key, text, ModifiersState::CONTROL),
                        Some(expected),
                    );
                }
                assert_eq!(
                    bindings.iter().any(|binding| matches!(
                        binding.action,
                        Action::CloneSplitRight | Action::CloneSplitDown
                    )),
                    use_splits
                );
            }
        }
    }

    #[test]
    fn quit_and_clear_defaults_require_exact_modifiers_and_safe_modes() {
        for bindings in [
            automexia_windows_key_bindings(true, true),
            automexia_unix_key_bindings(true, true),
        ] {
            for (key, mods, expected) in [
                (
                    "q",
                    ModifiersState::CONTROL | ModifiersState::SHIFT,
                    Action::Quit,
                ),
                (
                    "k",
                    ModifiersState::CONTROL | ModifiersState::ALT,
                    Action::ClearScreen,
                ),
            ] {
                let trigger = BindingKey::Keycode {
                    key: Key::Character(key.into()),
                    location: KeyLocation::Standard,
                };
                let matches = |mode: BindingMode, modifiers| {
                    bindings
                        .iter()
                        .filter(|binding| {
                            binding.is_triggered_by(mode.clone(), modifiers, &trigger)
                                && binding.action == expected
                        })
                        .count()
                };
                assert_eq!(matches(BindingMode::empty(), mods), 1);
                for mode in [
                    BindingMode::SEARCH,
                    BindingMode::VI,
                    BindingMode::ALT_SCREEN,
                ] {
                    assert_eq!(matches(mode, mods), 0);
                }
                for modifiers in [
                    ModifiersState::empty(),
                    ModifiersState::CONTROL,
                    ModifiersState::ALT,
                    ModifiersState::SHIFT,
                    mods | ModifiersState::SUPER,
                ] {
                    assert_eq!(matches(BindingMode::empty(), modifiers), 0);
                }
            }
        }
    }

    #[test]
    fn pane_creation_defaults_are_directional_and_leave_shell_controls_alone() {
        for bindings in [
            automexia_windows_key_bindings(true, true),
            automexia_unix_key_bindings(true, true),
        ] {
            for (key, modifiers, action) in [
                (
                    "r",
                    ModifiersState::ALT | ModifiersState::SHIFT,
                    Action::SplitRight,
                ),
                (
                    "d",
                    ModifiersState::ALT | ModifiersState::SHIFT,
                    Action::SplitDown,
                ),
                ("r", ModifiersState::ALT, Action::CloneSplitRight),
                ("d", ModifiersState::ALT, Action::CloneSplitDown),
            ] {
                assert_action_binding(
                    &bindings,
                    Key::Character(key.into()),
                    modifiers,
                    action,
                );
            }
        }
        let bindings =
            automexia_macos_key_bindings(true, true, ConfigKeyboard::default());
        for (key, action) in [
            ("r", Action::CloneSplitRight),
            ("d", Action::CloneSplitDown),
        ] {
            assert_action_binding(
                &bindings,
                Key::Character(key.into()),
                ModifiersState::SUPER | ModifiersState::ALT | ModifiersState::SHIFT,
                action,
            );
        }
    }

    #[test]
    fn explicit_clone_bindings_survive_default_and_reset_changes() {
        for (key, name, action) in [
            ("r", "CloneSplitRight", Action::CloneSplitRight),
            ("d", "CloneSplitDown", Action::CloneSplitDown),
        ] {
            let mut config = rio_backend::config::Config::default();
            config.bindings.keys.push(ConfigKeyBinding {
                key: key.into(),
                action: name.into(),
                with: "control".into(),
                esc: String::new(),
                mode: "~Search|~Vi".into(),
            });
            let unchanged = config.bindings.keys.clone();
            for _ in 0..2 {
                assert_action_binding(
                    &default_key_bindings(&config),
                    Key::Character(key.into()),
                    ModifiersState::CONTROL,
                    action.clone(),
                );
                assert_eq!(config.bindings.keys, unchanged);
            }
            config.bindings.keys.clear();
            let trigger = BindingKey::Keycode {
                key: Key::Character(key.into()),
                location: KeyLocation::Standard,
            };
            assert!(!default_key_bindings(&config).iter().any(|binding| binding
                .is_triggered_by(
                    BindingMode::empty(),
                    ModifiersState::CONTROL,
                    &trigger
                )));
        }
    }

    #[test]
    fn new_pane_chords_are_unique_mode_scoped_and_user_replaceable() {
        for defaults in [
            automexia_windows_key_bindings(true, true),
            automexia_unix_key_bindings(true, true),
        ] {
            for (key, modifiers) in [
                ("r", ModifiersState::ALT),
                ("d", ModifiersState::ALT),
                ("r", ModifiersState::ALT | ModifiersState::SHIFT),
                ("d", ModifiersState::ALT | ModifiersState::SHIFT),
            ] {
                let trigger = BindingKey::Keycode {
                    key: Key::Character(key.into()),
                    location: KeyLocation::Standard,
                };
                assert_eq!(
                    defaults
                        .iter()
                        .filter(|binding| binding.is_triggered_by(
                            BindingMode::empty(),
                            modifiers,
                            &trigger
                        ))
                        .count(),
                    1
                );
                for mode in [
                    BindingMode::SEARCH,
                    BindingMode::VI,
                    BindingMode::ALT_SCREEN,
                ] {
                    assert!(!defaults.iter().any(|binding| binding.is_triggered_by(
                        mode.clone(),
                        modifiers,
                        &trigger
                    )));
                }
                if matches!(key, "r" | "d") {
                    assert!(!defaults.iter().any(|binding| binding.is_triggered_by(
                        BindingMode::empty(),
                        ModifiersState::CONTROL | ModifiersState::SHIFT,
                        &trigger
                    )));
                }
            }
            let rebound = config_key_bindings(
                vec![ConfigKeyBinding {
                    key: "r".into(),
                    with: "alt".into(),
                    action: "ReceiveChar".into(),
                    esc: String::new(),
                    mode: String::new(),
                }],
                defaults,
            );
            assert_action_binding(
                &rebound,
                Key::Character("r".into()),
                ModifiersState::ALT,
                Action::ReceiveChar,
            );
            assert!(!rebound
                .iter()
                .any(|binding| binding.action == Action::CloneSplitRight));
            assert!(rebound
                .iter()
                .any(|binding| binding.action == Action::SplitRight));
        }
    }

    #[test]
    fn pane_mnemonics_have_no_retired_aliases_and_disabled_splits_restore_input() {
        use automexia_keybindings::PlatformFamily::{LinuxBsd, Windows};
        for platform in [Windows, LinuxBsd] {
            for enabled in [false, true] {
                let mut config = rio_backend::config::Config::default();
                config.navigation.use_split = enabled;
                let defaults = test_platform_defaults(&config, platform);
                for key in ["+", "=", "-", "_"] {
                    let trigger = BindingKey::Keycode {
                        key: Key::Character(key.into()),
                        location: KeyLocation::Standard,
                    };
                    assert!(!defaults.iter().any(|binding| binding.is_triggered_by(
                        BindingMode::empty(),
                        ModifiersState::ALT | ModifiersState::SHIFT,
                        &trigger,
                    )));
                }
                for (key, modifiers) in [
                    ("r", ModifiersState::ALT),
                    ("d", ModifiersState::ALT),
                    ("r", ModifiersState::ALT | ModifiersState::SHIFT),
                    ("d", ModifiersState::ALT | ModifiersState::SHIFT),
                ] {
                    let trigger = BindingKey::Keycode {
                        key: Key::Character(key.into()),
                        location: KeyLocation::Standard,
                    };
                    for mode in [
                        BindingMode::empty(),
                        BindingMode::SEARCH,
                        BindingMode::VI,
                        BindingMode::ALT_SCREEN,
                    ] {
                        assert_eq!(
                            defaults
                                .iter()
                                .filter(|binding| binding.is_triggered_by(
                                    mode.clone(),
                                    modifiers,
                                    &trigger,
                                ))
                                .count(),
                            usize::from(enabled && mode.is_empty())
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn function_keys_parse_in_config() {
        for (name, named) in [("f1", F1), ("f5", F5), ("f12", F12), ("f20", F20)] {
            let config_bindings = vec![ConfigKeyBinding {
                key: String::from(name),
                action: String::from("quit"),
                with: String::from(""),
                esc: String::from(""),
                mode: String::from(""),
            }];
            let new_bindings = config_key_bindings(config_bindings, Vec::new());
            assert_eq!(new_bindings.len(), 1, "{name} must parse");
            assert_eq!(
                new_bindings[0].trigger,
                BindingKey::Keycode {
                    key: Key::Named(named),
                    location: KeyLocation::Standard
                }
            );
        }
    }

    #[test]
    fn defaults_have_no_duplicate_triggers() {
        let config = rio_backend::config::Config::default();
        let bindings = default_key_bindings(&config);
        for (i, a) in bindings.iter().enumerate() {
            for b in bindings.iter().skip(i + 1) {
                assert!(
                    !(a.trigger == b.trigger
                        && a.mods == b.mods
                        && a.mode == b.mode
                        && a.notmode == b.notmode
                        && a.action == b.action),
                    "duplicate default binding: {a:?}"
                );
            }
        }
    }

    fn assert_no_overlapping_shortcuts(label: &str, bindings: &[KeyBinding]) {
        for (index, binding) in bindings.iter().enumerate() {
            for candidate in bindings.iter().skip(index + 1) {
                assert!(
                    !binding.triggers_match(candidate),
                    "{label} shortcut collision: {binding:?} overlaps {candidate:?}"
                );
            }
        }
    }

    fn assert_no_cross_table_overlaps(
        label: &str,
        inherited: &[KeyBinding],
        platform: &[KeyBinding],
    ) {
        for existing in inherited {
            for candidate in platform {
                assert!(
                    !existing.triggers_match(candidate),
                    "{label} shortcut collision: {existing:?} overlaps {candidate:?}"
                );
            }
        }
    }

    fn assert_action_binding(
        bindings: &[KeyBinding],
        key: Key,
        modifiers: ModifiersState,
        action: Action,
    ) {
        let trigger = BindingKey::Keycode {
            key,
            location: KeyLocation::Standard,
        };
        assert!(
            bindings.iter().any(|binding| {
                binding.trigger == trigger
                    && binding.mods == modifiers
                    && binding.action == action
            }),
            "missing shortcut {modifiers:?} + {trigger:?} => {action:?}"
        );
    }

    fn assert_feature_launcher_bindings(
        bindings: &[KeyBinding],
        primary_modifier: ModifiersState,
    ) {
        for (key, action) in [
            ("h", Action::OpenConnectionHub),
            ("o", Action::OpenActionCenter),
            ("m", Action::OpenExtensionMarketplace),
            ("l", Action::OpenFontBrowser),
        ] {
            let modifiers = primary_modifier | ModifiersState::SHIFT;
            assert_action_binding(
                bindings,
                Key::Character(key.into()),
                modifiers,
                action.clone(),
            );
            assert_terminal_modes_suppress_binding(
                bindings,
                Key::Character(key.into()),
                modifiers,
                action,
            );
        }
    }

    fn assert_command_jump_bindings(
        bindings: &[KeyBinding],
        primary_modifier: ModifiersState,
    ) {
        let modifiers = primary_modifier | ModifiersState::SHIFT;
        for (key, action) in [
            (Key::Named(ArrowUp), Action::ScrollToPrevPrompt),
            (Key::Named(ArrowDown), Action::ScrollToNextPrompt),
        ] {
            assert_action_binding(bindings, key.clone(), modifiers, action.clone());
            assert_terminal_modes_suppress_binding(bindings, key, modifiers, action);
        }
    }

    fn assert_terminal_modes_suppress_binding(
        bindings: &[KeyBinding],
        key: Key,
        modifiers: ModifiersState,
        action: Action,
    ) {
        let trigger = BindingKey::Keycode {
            key,
            location: KeyLocation::Standard,
        };
        let binding = bindings
            .iter()
            .find(|binding| {
                binding.trigger == trigger
                    && binding.mods == modifiers
                    && binding.action == action
            })
            .expect("mode-scoped application binding");
        assert!(binding.is_triggered_by(BindingMode::empty(), modifiers, &trigger));
        assert!(binding.notmode.contains(BindingMode::SEARCH));
        assert!(binding.notmode.contains(BindingMode::VI));
        assert!(binding.notmode.contains(BindingMode::ALT_SCREEN));
        for mode in [
            BindingMode::SEARCH,
            BindingMode::VI,
            BindingMode::ALT_SCREEN,
        ] {
            assert!(
                !binding.is_triggered_by(mode.clone(), modifiers, &trigger),
                "application shortcut {modifiers:?} + {trigger:?} leaked into {mode:?}"
            );
        }
    }

    #[test]
    fn windows_ctrl_v_pastes_and_user_can_restore_terminal_input() {
        let bindings = automexia_windows_key_bindings(true, true);
        for modifiers in [
            ModifiersState::CONTROL,
            ModifiersState::CONTROL | ModifiersState::SHIFT,
        ] {
            assert_action_binding(
                &bindings,
                Key::Character("v".into()),
                modifiers,
                Action::Paste,
            );
        }

        let overridden = config_key_bindings(
            vec![ConfigKeyBinding {
                key: "v".into(),
                action: "receivechar".into(),
                with: "control".into(),
                esc: String::new(),
                mode: String::new(),
            }],
            bindings,
        );
        let ctrl_v_trigger = BindingKey::Keycode {
            key: Key::Character("v".into()),
            location: KeyLocation::Standard,
        };
        let ctrl_v_bindings: Vec<_> = overridden
            .iter()
            .filter(|binding| {
                binding.trigger == ctrl_v_trigger
                    && binding.mods == ModifiersState::CONTROL
            })
            .collect();
        assert_eq!(ctrl_v_bindings.len(), 1);
        assert_eq!(ctrl_v_bindings[0].action, Action::ReceiveChar);
    }

    #[test]
    fn automexia_windows_defaults_restore_the_classic_workflow() {
        let config = rio_backend::config::Config::default();
        let inherited = key_bindings_with_platform(&config, |_, _, _| Vec::new());
        let bindings = automexia_windows_key_bindings(true, true);
        assert_no_overlapping_shortcuts("Windows", &bindings);
        assert_no_cross_table_overlaps("Windows", &inherited, &bindings);
        assert_feature_launcher_bindings(&bindings, ModifiersState::CONTROL);
        assert_command_jump_bindings(&bindings, ModifiersState::CONTROL);
        assert_action_binding(
            &bindings,
            Key::Character("n".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
            Action::WindowCreateNew,
        );
        assert_action_binding(
            &bindings,
            Key::Character("i".into()),
            ModifiersState::CONTROL | ModifiersState::ALT,
            Action::PreviewSelectedImage,
        );

        assert_action_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::CONTROL,
            Action::TabCreateNew,
        );
        assert_action_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
            Action::LocalTabCreateNew,
        );
        assert_action_binding(
            &bindings,
            Key::Character("r".into()),
            ModifiersState::ALT | ModifiersState::SHIFT,
            Action::SplitRight,
        );
        assert_action_binding(
            &bindings,
            Key::Character("d".into()),
            ModifiersState::ALT | ModifiersState::SHIFT,
            Action::SplitDown,
        );
        assert_action_binding(
            &bindings,
            Key::Named(ArrowLeft),
            ModifiersState::ALT,
            Action::SelectPaneLeft,
        );
        assert_action_binding(
            &bindings,
            Key::Named(ArrowDown),
            ModifiersState::ALT,
            Action::SelectPaneDown,
        );
        assert_action_binding(
            &bindings,
            Key::Named(PageUp),
            ModifiersState::ALT,
            Action::SelectPrevLocalTab,
        );
        assert_action_binding(
            &bindings,
            Key::Named(PageDown),
            ModifiersState::ALT,
            Action::SelectNextLocalTab,
        );
        assert_action_binding(
            &bindings,
            Key::Named(F4),
            ModifiersState::CONTROL,
            Action::TabCloseCurrent,
        );
        assert_action_binding(
            &bindings,
            Key::Named(F4),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
            Action::TabCloseUnfocused,
        );
        assert_terminal_modes_suppress_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::ALT | ModifiersState::SHIFT,
            Action::ToggleAppearanceTheme,
        );
    }

    #[test]
    fn automexia_unix_defaults_restore_the_classic_workflow() {
        let config = rio_backend::config::Config::default();
        let inherited = key_bindings_with_platform(&config, |_, _, _| Vec::new());
        let bindings = automexia_unix_key_bindings(true, true);
        assert_no_cross_table_overlaps("Unix", &inherited, &bindings);
        assert_feature_launcher_bindings(&bindings, ModifiersState::CONTROL);
        assert_command_jump_bindings(&bindings, ModifiersState::CONTROL);
        assert_action_binding(
            &bindings,
            Key::Character("i".into()),
            ModifiersState::CONTROL | ModifiersState::ALT,
            Action::PreviewSelectedImage,
        );
        let copy_actions = bindings
            .iter()
            .filter(|binding| {
                binding.mods == ModifiersState::CONTROL | ModifiersState::SHIFT
                    && binding.trigger
                        == BindingKey::Keycode {
                            key: Key::Character("c".into()),
                            location: KeyLocation::Standard,
                        }
            })
            .count();
        assert_eq!(
            copy_actions, 2,
            "Ctrl+Shift+C intentionally copies and clears the vi selection"
        );
        assert_action_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::CONTROL,
            Action::TabCreateNew,
        );
        assert_action_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
            Action::LocalTabCreateNew,
        );
        assert_action_binding(
            &bindings,
            Key::Character("r".into()),
            ModifiersState::ALT | ModifiersState::SHIFT,
            Action::SplitRight,
        );
        assert_action_binding(
            &bindings,
            Key::Named(ArrowRight),
            ModifiersState::ALT,
            Action::SelectPaneRight,
        );
        assert_action_binding(
            &bindings,
            Key::Named(ArrowUp),
            ModifiersState::ALT,
            Action::SelectPaneUp,
        );
        assert_action_binding(
            &bindings,
            Key::Named(PageUp),
            ModifiersState::ALT,
            Action::SelectPrevLocalTab,
        );
        assert_action_binding(
            &bindings,
            Key::Named(PageDown),
            ModifiersState::ALT,
            Action::SelectNextLocalTab,
        );
        assert_action_binding(
            &bindings,
            Key::Named(F4),
            ModifiersState::CONTROL,
            Action::TabCloseCurrent,
        );
        assert_action_binding(
            &bindings,
            Key::Named(F4),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
            Action::TabCloseUnfocused,
        );
        assert_action_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::ALT | ModifiersState::SHIFT,
            Action::ToggleAppearanceTheme,
        );
        assert_terminal_modes_suppress_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::ALT | ModifiersState::SHIFT,
            Action::ToggleAppearanceTheme,
        );
    }

    #[test]
    fn automexia_macos_defaults_restore_the_classic_workflow() {
        let config = rio_backend::config::Config::default();
        let inherited = key_bindings_with_platform(&config, |_, _, _| Vec::new());
        let bindings =
            automexia_macos_key_bindings(true, true, ConfigKeyboard::default());
        assert_no_cross_table_overlaps("macOS", &inherited, &bindings);
        assert_feature_launcher_bindings(&bindings, ModifiersState::SUPER);
        assert_command_jump_bindings(&bindings, ModifiersState::SUPER);
        assert_action_binding(
            &bindings,
            Key::Character("i".into()),
            ModifiersState::SUPER | ModifiersState::ALT,
            Action::PreviewSelectedImage,
        );
        assert_action_binding(
            &bindings,
            Key::Character("w".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT,
            Action::TabCloseCurrent,
        );
        assert_action_binding(
            &bindings,
            Key::Character("w".into()),
            ModifiersState::SUPER | ModifiersState::ALT,
            Action::TabCloseUnfocused,
        );
        assert_action_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::SUPER | ModifiersState::ALT | ModifiersState::SHIFT,
            Action::ToggleAppearanceTheme,
        );
        assert_terminal_modes_suppress_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::SUPER | ModifiersState::ALT | ModifiersState::SHIFT,
            Action::ToggleAppearanceTheme,
        );
        let command_k_actions = bindings
            .iter()
            .filter(|binding| {
                binding.mods == ModifiersState::SUPER
                    && binding.trigger
                        == BindingKey::Keycode {
                            key: Key::Character("k".into()),
                            location: KeyLocation::Standard,
                        }
            })
            .count();
        assert_eq!(
            command_k_actions, 2,
            "Cmd+K intentionally sends form-feed and clears saved history"
        );

        assert_action_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::SUPER,
            Action::TabCreateNew,
        );
        assert_action_binding(
            &bindings,
            Key::Character("d".into()),
            ModifiersState::SUPER,
            Action::SplitRight,
        );
        assert_action_binding(
            &bindings,
            Key::Character("d".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT,
            Action::SplitDown,
        );
        assert_action_binding(
            &bindings,
            Key::Character("t".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT,
            Action::LocalTabCreateNew,
        );
        assert_action_binding(
            &bindings,
            Key::Named(ArrowLeft),
            ModifiersState::SUPER | ModifiersState::ALT,
            Action::SelectPaneLeft,
        );
        assert_action_binding(
            &bindings,
            Key::Named(ArrowDown),
            ModifiersState::SUPER | ModifiersState::ALT,
            Action::SelectPaneDown,
        );
        assert_action_binding(
            &bindings,
            Key::Character("[".into()),
            ModifiersState::SUPER | ModifiersState::ALT,
            Action::SelectPrevLocalTab,
        );
        assert_action_binding(
            &bindings,
            Key::Character("]".into()),
            ModifiersState::SUPER | ModifiersState::ALT,
            Action::SelectNextLocalTab,
        );
    }

    #[test]
    fn up_arrow_remains_native_pty_input() {
        let config = rio_backend::config::Config::default();
        let bindings = default_key_bindings(&config);
        assert!(bindings.iter().any(|binding| {
            binding.mods.is_empty()
                && binding.trigger
                    == BindingKey::Keycode {
                        key: Key::Named(ArrowUp),
                        location: KeyLocation::Standard,
                    }
                && matches!(&binding.action, Action::Esc(value) if value == "\x1b[A")
        }));
    }

    #[test]
    fn bindings_alt_enter_conflict_resolution() {
        // Test Windows Alt+Enter conflict resolution
        let bindings = bindings!(
            KeyBinding;
            Key::Named(Enter), ModifiersState::ALT; Action::ToggleFullscreen;
        );

        // User wants to use Alt+Enter for a custom action
        let config_bindings = vec![ConfigKeyBinding {
            key: String::from("return"),
            action: String::from("scroll(1)"),
            with: String::from("alt"),
            esc: String::from(""),
            mode: String::from(""),
        }];

        let new_bindings = config_key_bindings(config_bindings, bindings);

        // Should have 1 binding (the original Alt+Enter should be replaced)
        assert_eq!(new_bindings.len(), 1);

        assert_eq!(&new_bindings[0].action, &Action::Scroll(1));
    }
}
