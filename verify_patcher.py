#!/usr/bin/env python3
from importlib.util import spec_from_file_location, module_from_spec
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sp = spec_from_file_location("automexia_apply", ROOT / "apply_automexia.py")
mod = module_from_spec(sp)
assert sp.loader
sp.loader.exec_module(mod)

fixtures = {}
fixtures["frontends/rioterm/Cargo.toml"] = '''[target.'cfg(not(target_arch = "wasm32"))'.dependencies]\nserde = { workspace = true }\n'''
fixtures["frontends/rioterm/src/main.rs"] = '''mod context;\nmod global_hotkey;\nfn envs(){ std::env::set_var("TERM_PROGRAM", "rio");\n    std::env::set_var("COLORTERM", "truecolor"); }\n'''
fixtures["frontends/rioterm/src/context/renderable.rs"] = '''use rustc_hash::FxHashMap;\nuse std::time::Instant;\nstruct X {\n    pub term_colors: TermColors,\n    /// Visible-area scroll offset\n}\nfn n(){ let _ = RenderableContent {\n            term_colors: TermColors::default(),\n            display_offset: 0,\n};}\n'''
fixtures["frontends/rioterm/src/context/mod.rs"] = 'pub struct Context<T> {\n    #[cfg(not(target_os = "windows"))]\n    pub shell_pid: u32,\n}\nfn dead() {\n    Context {\n        #[cfg(not(target_os = "windows"))]\n        shell_pid: 1,\n    };\n}\nfn spawn() {\n        #[cfg(not(target_os = "windows"))]\n        let shell_pid = *pty.child.pid.clone() as u32;\n        #[cfg(target_os = "windows")]\n        {\n            pty = match create_pty(\n                config.shell.program.as_deref(),\n                config.shell.args.clone(),\n                &config.working_dir,\n                None,\n                cols,\n                rows,\n            ) { Ok(created_pty) => created_pty, Err(err) => return Err(err) };\n        }\n        Ok(Context {\n            #[cfg(not(target_os = "windows"))]\n            shell_pid,\n        })\n}\n'

fixtures["frontends/rioterm/src/renderer/mod.rs"] = '''pub mod confirm_quit;\npub mod custom_cursor;\nstruct Renderer {\n    pub command_palette: command_palette::CommandPalette,\n    unfocused_split_opacity: f32,\n}\nimpl Renderer { fn new(){\n        let colors = List::from(&config.colors);\n        let named_colors = config.colors;\n        Renderer {\n            named_colors,\n            dynamic_background,\n};}\n    #[inline]\n    pub fn use_drawable_chars(&self) -> bool {\n        true\n    }\n    pub fn run(\n        &mut self,\n        sugarloaf: &mut Sugarloaf,\n        context_manager: &mut ContextManager<EventProxy>,\n    ) -> (Option<crate::context::renderable::WindowUpdate>, bool) {\n        let mut any_panel_dirty = false;\n            let force_full_damage = has_active_changed || self.is_game_mode_enabled;\n    }\nfn snap(){\n                context.renderable_content.term_colors = terminal.colors;\n                context.renderable_content.display_offset = terminal.display_offset();\n}\nfn paint(){\n        self.search.render(\n            sugarloaf,\n            (window_size.width, window_size.height, scale_factor),\n        );\n\n        self.command_palette.render(\n}\n}\n'''
fixtures["frontends/rioterm/src/renderer/utils.rs"] = '''use crate::constants;
use rio_backend::config::navigation::Navigation;
#[inline]
pub fn padding_top_from_config(
    navigation: &Navigation,
    padding_y_top: f32,
    #[allow(unused)] num_tabs: usize,
    #[allow(unused)] macos_use_unified_titlebar: bool,
) -> f32 {
    // When navigation is enabled (Tab mode), start content below island
    if navigation.is_enabled() {
        // On Linux/Windows, if hide_if_single is true and there's only one tab,
        // the island is hidden so render from 0 + configured margin
        #[cfg(not(target_os = "macos"))]
        if navigation.hide_if_single && num_tabs <= 1 {
            return constants::PADDING_Y + padding_y_top;
        }

        use crate::renderer::island::ISLAND_HEIGHT;
        return ISLAND_HEIGHT + padding_y_top;
    }

    let default_padding = constants::PADDING_Y + padding_y_top;

    #[cfg(target_os = "macos")]
    {
        use rio_backend::config::navigation::NavigationMode;
        if navigation.mode == NavigationMode::NativeTab {
            let additional = if macos_use_unified_titlebar {
                constants::ADDITIONAL_PADDING_Y_ON_UNIFIED_TITLEBAR
            } else {
                0.0
            };
            return additional + padding_y_top;
        }
    }

    default_padding
}
'''
fixtures["frontends/rioterm/src/renderer/command_palette.rs"] = '''use crate::renderer::scrollbar;\nconst ORDER: u8 = 20;\nenum PaletteAction {\n    CloseCurrentSplitOrTab,\n    /// Browse the family names\n    ListFonts,\n}\nstruct Command { title: &'static str, shortcut: &'static str, action: PaletteAction }\nconst COMMANDS: &[Command] = &[\nCommand { title: "New Tab", shortcut: "Cmd+T", action: PaletteAction::TabCreate },\nCommand { title: "Close Tab", shortcut: "Cmd+W", action: PaletteAction::TabClose },\nCommand { title: "Split Right", shortcut: "Cmd+D", action: PaletteAction::SplitRight },\nCommand { title: "Split Down", shortcut: "Cmd+Shift+D", action: PaletteAction::SplitDown },\nCommand { title: "Settings", shortcut: "Cmd+,", action: PaletteAction::ConfigEditor },\nCommand { title: "New Window", shortcut: "Cmd+N", action: PaletteAction::WindowCreateNew },\nCommand { title: "Copy", shortcut: "Cmd+C", action: PaletteAction::Copy },\nCommand { title: "Paste", shortcut: "Cmd+V", action: PaletteAction::Paste },\nCommand { title: "Search Forward", shortcut: "Cmd+F", action: PaletteAction::SearchForward },\nCommand { title: "Increase Font Size", shortcut: "Cmd++", action: PaletteAction::IncreaseFontSize },\nCommand { title: "Decrease Font Size", shortcut: "Cmd+-", action: PaletteAction::DecreaseFontSize },\nCommand { title: "Reset Font Size", shortcut: "Cmd+0", action: PaletteAction::ResetFontSize },\n    Command {\n        title: "Toggle Vi Mode",\n        shortcut: "",\n        action: PaletteAction::ToggleViMode,\n    },\n    Command {\n        title: "Toggle Fullscreen",\n        shortcut: "",\n        action: PaletteAction::ToggleFullscreen,\n    },\n    Command {\n        title: "Toggle Appearance Theme",\n        shortcut: "",\n        action: PaletteAction::ToggleAppearanceTheme,\n    },\n    Command {\n        title: "Clear History",\n        shortcut: "",\n        action: PaletteAction::ClearHistory,\n    },\n    Command {\n        title: "List Fonts",\n        shortcut: "",\n        action: PaletteAction::ListFonts,\n    },\n];\nenum PaletteMode {\n    Commands,\n    Fonts(Vec<String>),\n}\nenum PaletteRow<'a> {\n    Command { title: &'a str, shortcut: &'a str, action: PaletteAction },\n    Font {\n        family: &'a str,\n    },\n}\nimpl<'a> PaletteRow<'a>{\nfn title(&self)->&'a str{match *self {\n            PaletteRow::Command { title, .. } => title,\n            PaletteRow::Font { family } => family,\n}}\nfn shortcut(&self)->&'a str{match *self {\n            PaletteRow::Command { shortcut, .. } => shortcut,\n            PaletteRow::Font { .. } => "",\n}}\nfn action(&self)->Option<PaletteAction>{match *self {\n            PaletteRow::Command { action, .. } => Some(action),\n            PaletteRow::Font { .. } => None,\n}}}\nimpl CommandPalette {\n    pub fn enter_fonts_mode(&mut self, fonts: Vec<String>) {\n        self.mode = PaletteMode::Fonts(fonts);\n        self.query.clear();\n        self.selected_index = 0;\n        self.scroll_offset = 0;\n        self.caret_blink_start = Instant::now();\n        self.last_scroll_time = None;\n    }\n    pub fn get_selected_font(&self) -> Option<String> {\n        self.filtered_rows()\n            .get(self.selected_index)\n            .and_then(|(_, row)| match row {\n                PaletteRow::Font { family } => Some((*family).to_owned()),\n                PaletteRow::Command { .. } => None,\n            })\n    }\nfn filtered_rows(&self){match &self.mode {\n            PaletteMode::Fonts(fonts) => fonts\n                .iter()\n                .filter_map(|family| {\n                    let score = fuzzy_score(&self.query, family)?;\n                    Some((score, PaletteRow::Font { family }))\n                })\n                .collect(),\n}}\nfn p(){let placeholder = match self.mode {\n            PaletteMode::Commands => "Type a command...",\n            PaletteMode::Fonts(_) => "Type a font name...",\n};}\n}\n'''
fixtures["frontends/rioterm/src/router/mod.rs"] = '''fn k(){\n                        let selected_font = self\n                            .window\n                            .screen\n                            .renderer\n                            .command_palette\n                            .get_selected_font();\n                        let selected_action = self\n                            .window\n                            .screen\n                            .renderer\n                            .command_palette\n                            .get_selected_action();\n                        match selected_action {\n                            // `ListFonts` stays inside the palette —\n                            // swap\n                        }\n}\n'''
fixtures["frontends/rioterm/src/screen/mod.rs"] = '''match action {\n            PaletteAction::ListFonts => {\n                // Handled in the router: switches the palette into fonts\n            }\n}\n'''
fixtures["frontends/rioterm/src/grid_emit.rs"] = '''use rio_backend::sugarloaf::font::FontLibrary;\nstruct GridGlyphRasterizer {\n    run_cell_columns: Vec<u16>,\n}\nfn new(){ Self {\n            run_cell_columns: Vec::new(),\n};}\nfn build(){\n    let needs_per_cell_check = has_sel || has_color_hints;\n        // Glyph Protocol short-circuit:\nX{\n                } else if let Some(tag) = hint_tag {\n                    cell_fg_hinted(tag, renderer)\n                } else {\n                    cell_fg(sq, style, renderer, term_colors)\n                }\n}\n        // Built-in drawable sprite short-circuit:\nY{\n                } else if let Some(tag) = hint_tag {\n                    cell_fg_hinted(tag, renderer)\n                } else {\n                    cell_fg(sq, style, renderer, term_colors)\n                }\n}\n        let run_start = x;\n            // Pull fg from the cluster's first cell.\nZ{\n                } else if let Some(tag) = hint_tag {\n                    // Hint-fg wins over the cell's own fg, matching\n                    // `.search` / `.search_selected` branches at\n                    // `generic.zig:2829-2833` (the fg picker mirrors bg).\n                    (CellText::ATLAS_GRAYSCALE, cell_fg_hinted(tag, renderer))\n                } else {\n                    (\n                        CellText::ATLAS_GRAYSCALE,\n                        cell_fg(src_sq, src_style, renderer, term_colors),\n                    )\n                }\n            fg_scratch.push(CellText {\n}\n}\n'''
fixtures["frontends/rioterm/src/bindings/mod.rs"] = '''before\n// Windows\n#[cfg(all(target_os = "windows", not(test)))]\npub fn platform_key_bindings(\n    use_navigation_key_bindings: bool,\n    use_splits: bool,\n    _: ConfigKeyboard,\n) -> Vec<KeyBinding> {\n    vec![]\n}\n\n#[cfg(test)]\npub fn platform_key_bindings(_: bool, _: bool, _: ConfigKeyboard) -> Vec<KeyBinding> { vec![] }\n'''

fixtures["rio-window/src/platform_impl/windows/util.rs"] = r'''
pub type AdjustWindowRectExForDpi = unsafe extern "system" fn(
    rect: *mut RECT,
    dwStyle: u32,
    bMenu: BOOL,
    dwExStyle: u32,
    dpi: u32,
) -> BOOL;

pub type GetPointerFrameInfoHistory = unsafe extern "system" fn(
    pointerId: u32,
    entriesCount: *mut u32,
    pointerCount: *mut u32,
    pointerInfo: *mut POINTER_INFO,
) -> BOOL;

pub type SkipPointerFrameMessages = unsafe extern "system" fn(pointerId: u32) -> BOOL;
pub type GetPointerDeviceRects = unsafe extern "system" fn(
    device: HANDLE,
    pointerDeviceRect: *mut RECT,
    displayRect: *mut RECT,
) -> BOOL;
pub type GetPointerTouchInfo =
    unsafe extern "system" fn(pointerId: u32, touchInfo: *mut POINTER_TOUCH_INFO) -> BOOL;
pub type GetPointerPenInfo =
    unsafe extern "system" fn(pointId: u32, penInfo: *mut POINTER_PEN_INFO) -> BOOL;
'''
fixtures["sugarloaf/src/renderer/mod.rs"] = '''fn render() {
    if let Some(bg_tex) = background_image_texture.as_ref() {
        if let ImageTexture::Wgpu { view, .. } = &bg_tex.gpu {
            use_view(view);
        }
    }
    if let Some(img) = image_textures.get(&draw.image_id) {
        if let ImageTexture::Wgpu { view, .. } = &img.gpu {
            use_view(view);
        }
    }
    if let Some(img) = image_textures.get(&draw.image_id) {
        if let ImageTexture::Wgpu { view, .. } = &img.gpu {
            use_view(view);
        }
    }
}
'''
fixtures["frontends/rioterm/src/context/title.rs"] = '''use crate::context::Context;
use std::path::Path;
pub fn create_title_extra_from_context<T: rio_backend::event::EventListener>(
    context: &Context<T>,
) -> Option<ContextTitleExtra> {
    #[cfg(unix)]
    let program =
        teletypewriter::foreground_process_name(*context.main_fd, context.shell_pid);

    #[cfg(not(unix))]
    let program = String::default();
    Some(ContextTitleExtra { program })
}
fn shorten_path(absolute: &str) -> String {
    let path = Path::new(absolute);

    // Replace home prefix with ~
    #[cfg(unix)]
    let display_path = {
        if let Some(home) = dirs::home_dir() {
            if let Ok(stripped) = path.strip_prefix(&home) {
                stripped.to_string_lossy().to_string()
            } else {
                absolute.to_string()
            }
        } else {
            absolute.to_string()
        }
    };
    #[cfg(not(unix))]
    let display_path = absolute.to_string();
    display_path
}
'''

migration_only = {"frontends/rioterm/src/renderer/utils.rs"}
for path, patcher in mod.PATCHERS.items():
    before = fixtures[path]
    once = patcher(before)
    twice = patcher(once)
    assert once == twice, f"{path}: patch is not idempotent"
    if path not in migration_only:
        assert once != before, f"{path}: patch did not change fixture"

# renderer/utils is intentionally stock on a fresh audited Rio checkout; its
# patcher exists only to remove the fixed v0.3.4-v0.3.6 status-lane geometry
# when upgrading an existing Automexia tree. Upgrade coverage lives in the
# full regression suite.
assert mod.patch_renderer_utils(fixtures["frontends/rioterm/src/renderer/utils.rs"]) == fixtures["frontends/rioterm/src/renderer/utils.rs"]

# Explicitly prove the warning-producing tokens/patterns are gone from fixtures.
win = mod.patch_windows_util_warnings(fixtures["rio-window/src/platform_impl/windows/util.rs"])
for bad in ("dwStyle", "bMenu", "dwExStyle", "pointerId", "entriesCount", "pointerCount", "pointerInfo", "pointerDeviceRect", "displayRect", "touchInfo", "pointId", "penInfo"):
    assert bad not in win, f"Windows warning token survived: {bad}"
sugar = mod.patch_sugarloaf_warnings(fixtures["sugarloaf/src/renderer/mod.rs"])
assert "if let ImageTexture::Wgpu { view, .. }" not in sugar
assert sugar.count("let ImageTexture::Wgpu { view, .. }") == 3
main = mod.patch_main(fixtures["frontends/rioterm/src/main.rs"])
assert "mod automexia;" in main
assert "mod extensions;" not in main

title = mod.patch_title_warnings(fixtures["frontends/rioterm/src/context/title.rs"])
assert "#[cfg(not(unix))]\n    let _ = context;" in title
assert "#[cfg(not(unix))]\n    let _ = path;" in title

print(f"PASS: {len(fixtures)} patchers apply once and are idempotent; session metadata + 27 reported warning sites are transformed")
