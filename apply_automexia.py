#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import re
import shutil
import subprocess
from pathlib import Path

UPSTREAM_TARGET = "7d595af583f6ef1ea6036a66b367ba1e5a84d4a2"

PATCHED_FILES = [
    "frontends/rioterm/Cargo.toml",
    "frontends/rioterm/src/main.rs",
    "frontends/rioterm/src/context/renderable.rs",
    "frontends/rioterm/src/context/mod.rs",
    "frontends/rioterm/src/renderer/mod.rs",
    "frontends/rioterm/src/renderer/utils.rs",
    "frontends/rioterm/src/renderer/command_palette.rs",
    "frontends/rioterm/src/router/mod.rs",
    "frontends/rioterm/src/screen/mod.rs",
    "frontends/rioterm/src/grid_emit.rs",
    "frontends/rioterm/src/bindings/mod.rs",
    "rio-window/src/platform_impl/windows/util.rs",
    "sugarloaf/src/renderer/mod.rs",
    "frontends/rioterm/src/context/title.rs",
]

OBSOLETE_FILES = [
    "frontends/rioterm/src/extensions/mod.rs",
    "frontends/rioterm/src/extensions/manager.rs",
    "frontends/rioterm/src/extensions/devops.rs",
    # v0.3.0 used one monolithic file. v0.3.1 splits discovery/model/semantics
    # under builtins/devops/; keeping both forms would make Rust reject the module.
    "frontends/rioterm/src/automexia/builtins/devops.rs",
]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text or contains_ignoring_whitespace(text, new):
        return text
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one marker, found {count}")
    return text.replace(old, new, 1)


def contains_ignoring_whitespace(text: str, fragment: str) -> bool:
    """Recognize generated Rust after rustfmt has changed line wrapping."""
    return re.sub(r"\s+", "", fragment) in re.sub(r"\s+", "", text)


def regex_replace_once(text: str, pattern: str, replacement: str, label: str) -> str:
    new_text, count = re.subn(pattern, replacement, text, count=1, flags=re.DOTALL)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one regex match, found {count}")
    return new_text


def patch_cargo(text: str) -> str:
    if 'serde_json = "1.0"' in text:
        return text
    return replace_once(
        text,
        "serde = { workspace = true }\n",
        'serde = { workspace = true }\nserde_json = "1.0"\n',
        "rioterm serde_json dependency",
    )


def patch_main(text: str) -> str:
    if not re.search(r"(?m)^mod automexia;$", text):
        if re.search(r"(?m)^mod extensions;$", text):
            text = re.sub(r"(?m)^mod extensions;$", "mod automexia;", text, count=1)
        else:
            text = replace_once(
                text,
                "mod context;\n",
                "mod context;\nmod automexia;\n",
                "main Automexia platform module",
            )
    text = text.replace('std::env::set_var("TERM_PROGRAM", "rio");', 'std::env::set_var("TERM_PROGRAM", "Automexia");')
    marker = 'std::env::set_var("COLORTERM", "truecolor");'
    target_env = marker + '''\n    std::env::set_var("AUTOMEXIA_SHELL_INTEGRATION", "1");
    #[cfg(target_os = "windows")]
    {
        // Carry Automexia's identity through wsl.exe without changing
        // distro prompts for terminals launched outside Automexia.
        let mut wslenv = std::env::var("WSLENV").unwrap_or_default();
        for entry in ["TERM_PROGRAM/u", "AUTOMEXIA_SHELL_INTEGRATION/u", "COLORTERM/u"] {
            if !wslenv.split(':').any(|part| part == entry) {
                if !wslenv.is_empty() { wslenv.push(':'); }
                wslenv.push_str(entry);
            }
        }
        std::env::set_var("WSLENV", wslenv);
    }'''
    env_marker = 'std::env::set_var("AUTOMEXIA_SHELL_INTEGRATION", "1");'
    if env_marker not in text and marker in text:
        text = replace_once(text, marker, target_env, "Automexia shell integration environment")
    return text


def patch_renderable(text: str) -> str:
    text = replace_once(
        text,
        "use rustc_hash::FxHashMap;\nuse std::time::Instant;",
        "use rustc_hash::FxHashMap;\nuse std::path::PathBuf;\nuse std::time::Instant;",
        "renderable PathBuf import",
    )

    target_field = (
        "    pub term_colors: TermColors,\n"
        "    /// Current working directory captured under the same terminal lock as the visible rows.\n"
        "    /// Extension/UI code reads this cached value and never re-locks the PTY state during paint.\n"
        "    pub current_directory: Option<PathBuf>,\n"
        "    /// Raw terminal/OSC title captured with the same snapshot. Unlike the\n"
        "    /// configurable window title, this preserves child-session metadata\n"
        "    /// such as the standard WSL `user@host:/path` title.\n"
        "    pub terminal_title: String,\n"
        "    /// Optional shell-published distro metadata from OSC 1337 SetUserVar.\n"
        "    pub shell_distro: Option<String>,\n"
        "    /// Optional shell-published OS version metadata.\n"
        "    pub shell_os_version: Option<String>,\n"
        "    /// Whether Automexia shell integration announced itself for this session.\n"
        "    pub shell_integration: bool,\n"
        "    /// Whether the shell is currently waiting for editable prompt input.\n"
        "    pub shell_prompt_active: bool,\n"
        "    /// Visible-area scroll offset"
    )
    legacy_v0310 = (
        "    pub term_colors: TermColors,\n"
        "    /// Current working directory captured under the same terminal lock as the visible rows.\n"
        "    /// Extension/UI code reads this cached value and never re-locks the PTY state during paint.\n"
        "    pub current_directory: Option<PathBuf>,\n"
        "    /// Raw terminal/OSC title captured with the same snapshot. Unlike the\n"
        "    /// configurable window title, this preserves child-session metadata\n"
        "    /// such as the standard WSL `user@host:/path` title.\n"
        "    pub terminal_title: String,\n"
        "    /// Optional shell-published distro metadata from OSC 1337 SetUserVar.\n"
        "    pub shell_distro: Option<String>,\n"
        "    /// Optional shell-published OS version metadata.\n"
        "    pub shell_os_version: Option<String>,\n"
        "    /// Visible-area scroll offset"
    )
    legacy_v037 = (
        "    pub term_colors: TermColors,\n"
        "    /// Current working directory captured under the same terminal lock as the visible rows.\n"
        "    /// Extension/UI code reads this cached value and never re-locks the PTY state during paint.\n"
        "    pub current_directory: Option<PathBuf>,\n"
        "    /// Raw terminal/OSC title captured with the same snapshot. Unlike the\n"
        "    /// configurable window title, this preserves child-session metadata\n"
        "    /// such as the standard WSL `user@host:/path` title.\n"
        "    pub terminal_title: String,\n"
        "    /// Visible-area scroll offset"
    )
    legacy_v031 = (
        "    pub term_colors: TermColors,\n"
        "    /// Current working directory captured under the same terminal lock as the visible rows.\n"
        "    /// Extension/UI code reads this cached value and never re-locks the PTY state during paint.\n"
        "    pub current_directory: Option<PathBuf>,\n"
        "    /// Visible-area scroll offset"
    )
    if target_field not in text:
        if legacy_v0310 in text:
            text = text.replace(legacy_v0310, target_field, 1)
        elif legacy_v037 in text:
            text = text.replace(legacy_v037, target_field, 1)
        elif legacy_v031 in text:
            text = text.replace(legacy_v031, target_field, 1)
        else:
            text = replace_once(
                text,
                "    pub term_colors: TermColors,\n    /// Visible-area scroll offset",
                target_field,
                "renderable session metadata fields",
            )

    target_init = (
        "            term_colors: TermColors::default(),\n"
        "            current_directory: None,\n"
        "            terminal_title: String::new(),\n"
        "            shell_distro: None,\n"
        "            shell_os_version: None,\n"
        "            shell_integration: false,\n"
        "            shell_prompt_active: false,\n"
        "            display_offset: 0,"
    )
    legacy_v0310_init = (
        "            term_colors: TermColors::default(),\n"
        "            current_directory: None,\n"
        "            terminal_title: String::new(),\n"
        "            shell_distro: None,\n"
        "            shell_os_version: None,\n"
        "            display_offset: 0,"
    )
    legacy_v037_init = (
        "            term_colors: TermColors::default(),\n"
        "            current_directory: None,\n"
        "            terminal_title: String::new(),\n"
        "            display_offset: 0,"
    )
    legacy_v031_init = (
        "            term_colors: TermColors::default(),\n"
        "            current_directory: None,\n"
        "            display_offset: 0,"
    )
    if target_init not in text:
        if legacy_v0310_init in text:
            text = text.replace(legacy_v0310_init, target_init, 1)
        elif legacy_v037_init in text:
            text = text.replace(legacy_v037_init, target_init, 1)
        elif legacy_v031_init in text:
            text = text.replace(legacy_v031_init, target_init, 1)
        else:
            text = replace_once(
                text,
                "            term_colors: TermColors::default(),\n            display_offset: 0,",
                target_init,
                "renderable session metadata init",
            )
    return text

def patch_context_session(text: str) -> str:
    """Expose generic shell/session metadata without assuming a Windows Pty PID API."""
    changes = [
        (
            '    #[cfg(not(target_os = "windows"))]\n    pub shell_pid: u32,',
            '    pub shell_pid: u32,',
            "context shell PID field",
        ),
        (
            '        #[cfg(not(target_os = "windows"))]\n        shell_pid: 1,',
            '        shell_pid: 1,',
            "dead context shell PID",
        ),
        (
            '            #[cfg(not(target_os = "windows"))]\n            shell_pid,',
            '            shell_pid,',
            "context shell PID init",
        ),
    ]
    for old, new, label in changes:
        count = text.count(old)
        if count == 1:
            text = text.replace(old, new, 1)
            continue
        if count > 1:
            raise RuntimeError(f"{label}: expected one marker, found {count}")
        if not contains_ignoring_whitespace(text, new):
            raise RuntimeError(f"{label}: neither legacy nor normalized marker found")

    # Unix Rio exposes the real child PID. Windows ConPTY's `Pty` does not
    # expose a child PID at this audited revision, so preserve the Unix capture
    # and add a neutral 0 sentinel on Windows. Session-aware WSL discovery uses
    # terminal metadata (route/title/cwd), not process spawning or PID probing.
    unix_capture = (
        '        #[cfg(not(target_os = "windows"))]\n'
        '        let shell_pid = *pty.child.pid.clone() as u32;'
    )
    windows_capture = (
        unix_capture
        + '\n        #[cfg(target_os = "windows")]\n'
        + '        let shell_pid = 0u32;'
    )
    if not contains_ignoring_whitespace(text, windows_capture):
        if text.count(unix_capture) != 1:
            raise RuntimeError("spawned context shell PID capture: expected one Unix marker")
        text = text.replace(unix_capture, windows_capture, 1)

    old_call = """            pty = match create_pty(
                config.shell.program.as_deref(),
                config.shell.args.clone(),
                &config.working_dir,
                None,
                cols,
                rows,
            ) {"""
    new_call = """            let automexia_shell_program =
                crate::automexia::shell::normalized_program(config.shell.program.as_deref());
            let automexia_shell_args = crate::automexia::shell::normalized_args(
                automexia_shell_program.as_deref(),
                &config.shell.args,
            );
            pty = match create_pty(
                automexia_shell_program.as_deref(),
                automexia_shell_args,
                &config.working_dir,
                None,
                cols,
                rows,
            ) {"""
    previous_call = """            let automexia_shell_args = crate::automexia::shell::normalized_args(
                config.shell.program.as_deref(),
                &config.shell.args,
            );
            pty = match create_pty(
                config.shell.program.as_deref(),
                automexia_shell_args,
                &config.working_dir,
                None,
                cols,
                rows,
            ) {"""
    normalized_launch_present = all(
        marker in text
        for marker in (
            "let automexia_shell_program",
            "crate::automexia::shell::normalized_program(",
            "let automexia_shell_args",
            "automexia_shell_program.as_deref(),",
        )
    )
    if not normalized_launch_present:
        if previous_call in text:
            text = text.replace(previous_call, new_call, 1)
        elif old_call in text:
            text = text.replace(old_call, new_call, 1)
        elif "create_pty(" in text:
            raise RuntimeError("Windows shell launch normalization marker missing")

    return text


def patch_renderer(text: str) -> str:
    # Automexia owns the application palette. Named/ANSI colors are mapped
    # through one cross-shell theme so PowerShell, Bash, Zsh, WSL and macOS
    # share the same visual language. Explicit application truecolor remains
    # untouched by the normal renderer path.
    stock_colors = "        let colors = List::from(&config.colors);\n        let named_colors = config.colors;"
    themed_colors = "        let named_colors = crate::automexia::theme::effective_colors(config.colors);\n        let colors = List::from(&named_colors);"
    if not contains_ignoring_whitespace(text, themed_colors):
        if text.count(stock_colors) != 1:
            raise RuntimeError("renderer unified palette: expected one color initializer")
        text = text.replace(stock_colors, themed_colors, 1)

    # Migrate v0.2 call sites into the v0.3 Automexia ownership boundary before
    # applying any new renderer transforms. This makes upgrades idempotent and
    # removes the legacy `crate::extensions` namespace from compiled code.
    text = text.replace("crate::extensions::manager::", "crate::automexia::runtime::")
    text = text.replace("crate::extensions::devops::ID", "crate::automexia::builtins::devops::ID")
    text = replace_once(
        text,
        "pub mod confirm_quit;\npub mod custom_cursor;",
        "pub mod confirm_quit;\npub mod custom_cursor;\npub mod devops_status;",
        "renderer devops module",
    )
    text = replace_once(
        text,
        "    pub command_palette: command_palette::CommandPalette,\n    unfocused_split_opacity:",
        "    pub command_palette: command_palette::CommandPalette,\n    pub devops_enabled: bool,\n    extension_generation: u32,\n    pub devops_status: devops_status::DevOpsStatus,\n    unfocused_split_opacity:",
        "renderer extension fields",
    )
    if "devops_status: devops_status::DevOpsStatus::default()," not in text:
        text = replace_once(
            text,
            "            named_colors,\n            dynamic_background,",
            "            devops_enabled: crate::automexia::runtime::is_installed(crate::automexia::builtins::devops::ID),\n            extension_generation: crate::automexia::runtime::generation(),\n            devops_status: devops_status::DevOpsStatus::default(),\n            named_colors,\n            dynamic_background,",
            "renderer extension initializer",
        )
    method_marker = "    #[inline]\n    pub fn use_drawable_chars(&self) -> bool {\n"
    method = '''    /// Synchronize the cached extension activation state. The fast path is one\n    /// atomic generation load; filesystem state is never checked per frame.\n    pub fn sync_extension_state(&mut self) -> bool {\n        let generation = crate::automexia::runtime::generation();\n        if generation == self.extension_generation {\n            return false;\n        }\n        self.extension_generation = generation;\n        let enabled = crate::automexia::runtime::is_installed(crate::automexia::builtins::devops::ID);\n        let changed = enabled != self.devops_enabled;\n        self.devops_enabled = enabled;\n        if !enabled {\n            self.devops_status.clear();\n        }\n        changed\n    }\n\n    #[inline]\n    pub fn use_drawable_chars(&self) -> bool {\n'''
    if "pub fn sync_extension_state(&mut self) -> bool {" not in text:
        text = replace_once(text, method_marker, method, "renderer sync method")

    text = replace_once(
        text,
        "    ) -> (Option<crate::context::renderable::WindowUpdate>, bool) {\n        let mut any_panel_dirty = false;",
        "    ) -> (Option<crate::context::renderable::WindowUpdate>, bool) {\n        let extension_state_changed = self.sync_extension_state();\n        let mut any_panel_dirty = false;",
        "renderer extension state at frame start",
    )
    text = replace_once(
        text,
        "            let force_full_damage = has_active_changed || self.is_game_mode_enabled;",
        "            let force_full_damage =\n                has_active_changed || self.is_game_mode_enabled || extension_state_changed;",
        "renderer extension full damage",
    )

    snapshot_stock = (
        "                context.renderable_content.term_colors = terminal.colors;\n"
        "                context.renderable_content.display_offset = terminal.display_offset();"
    )
    snapshot_v031 = (
        "                context.renderable_content.term_colors = terminal.colors;\n"
        "                context.renderable_content.current_directory = terminal.current_directory.clone();\n"
        "                context.renderable_content.display_offset = terminal.display_offset();"
    )
    snapshot_v032 = (
        "                context.renderable_content.term_colors = terminal.colors;\n"
        "                context.renderable_content.current_directory = terminal.current_directory.clone();\n"
        "                context.renderable_content.terminal_title = terminal.title.to_string();\n"
        "                context.renderable_content.display_offset = terminal.display_offset();"
    )
    snapshot_v0310 = (
        "                context.renderable_content.term_colors = terminal.colors;\n"
        "                context.renderable_content.current_directory = terminal.current_directory.clone();\n"
        "                context.renderable_content.terminal_title = terminal.title.to_string();\n"
        "                context.renderable_content.shell_distro = terminal.user_vars.get(\"automexia_distro\").cloned();\n"
        "                context.renderable_content.shell_os_version = terminal.user_vars.get(\"automexia_os_version\").cloned();\n"
        "                context.renderable_content.display_offset = terminal.display_offset();"
    )
    snapshot_v038 = (
        "                context.renderable_content.term_colors = terminal.colors;\n"
        "                context.renderable_content.current_directory = terminal.current_directory.clone();\n"
        "                context.renderable_content.terminal_title = terminal.title.to_string();\n"
        "                context.renderable_content.shell_distro = terminal.user_vars.get(\"automexia_distro\").cloned();\n"
        "                context.renderable_content.shell_os_version = terminal.user_vars.get(\"automexia_os_version\").cloned();\n"
        "                context.renderable_content.shell_integration = terminal.user_vars.get(\"automexia_shell\").is_some_and(|value| value == \"1\");\n"
        "                context.renderable_content.shell_prompt_active = terminal.user_vars.get(\"automexia_prompt_active\").is_some_and(|value| value == \"1\");\n"
        "                context.renderable_content.display_offset = terminal.display_offset();"
    )
    if not contains_ignoring_whitespace(text, snapshot_v038):
        if snapshot_v0310 in text:
            text = text.replace(snapshot_v0310, snapshot_v038, 1)
        elif snapshot_v032 in text:
            text = text.replace(snapshot_v032, snapshot_v038, 1)
        elif snapshot_v031 in text:
            text = text.replace(snapshot_v031, snapshot_v038, 1)
        else:
            text = replace_once(
                text, snapshot_stock, snapshot_v038, "snapshot terminal session metadata"
            )

    # DevOps context belongs to semantic prompt rows, not fixed window chrome.
    # Shell integration creates one blank OSC-133 Prompt row followed by an
    # editable PromptContinuation row. Rio already carries SemanticPrompt with
    # rows into scrollback; the application maps only blank prompt rows to
    # renderer-neutral PromptAnchors, keeping DevOps out of the terminal core.
    search_marker = '''        self.search.render(
            sugarloaf,
            (window_size.width, window_size.height, scale_factor),
        );
'''
    command_marker = '''        self.command_palette.render(
'''
    search_at = text.find(search_marker)
    if search_at < 0:
        raise RuntimeError("renderer DevOps semantic-prompt HUD: search overlay marker missing")
    command_at = text.find(command_marker, search_at + len(search_marker))
    if command_at < 0:
        raise RuntimeError("renderer DevOps semantic-prompt HUD: command palette marker missing")

    prompt_hud = '''        self.search.render(
            sugarloaf,
            (window_size.width, window_size.height, scale_factor),
        );

        if self.devops_enabled {
            let (session, prompt_active, historical_anchors, live_anchor) = {
                let grid = context_manager.current_grid();
                let (context, margin) = grid.current_context_with_computed_dimension();
                let rc = &context.renderable_content;
                let scale = scale_factor.max(f32::EPSILON);
                let cell_height = context.dimension.cell.cell_height as f32 / scale;
                let cell_width = context.dimension.cell.cell_width as f32 / scale;
                let origin_x = margin.left / scale;
                let origin_y = margin.top / scale;
                let grid_width = rc.columns.max(1) as f32 * cell_width;
                let first_absolute_row = rc.lines_evicted.saturating_add(
                    rc.history_size.saturating_sub(rc.display_offset) as u64,
                );

                let historical_anchors = rc
                    .visible_rows
                    .iter()
                    .enumerate()
                    .filter_map(|(row_index, row)| {
                        if row.semantic_prompt
                            != rio_backend::crosswords::grid::row::SemanticPrompt::Prompt
                        {
                            return None;
                        }
                        let blank = row.inner.iter().all(|sq| {
                            sq.is_bg_only() || matches!(sq.c(), '\\0' | ' ')
                        });
                        if !blank { return None; }
                        Some(crate::automexia::ui::PromptAnchor {
                            key: first_absolute_row.saturating_add(row_index as u64),
                            x: origin_x,
                            y: origin_y + row_index as f32 * cell_height,
                            width: grid_width,
                            height: cell_height,
                        })
                    })
                    .collect::<Vec<_>>();

                // The shell marks the editable command row with OSC-133
                // PromptContinuation. The DevOps row is the blank Prompt row
                // immediately above it. Resolve that pair on every frame so
                // resize/reflow follows terminal semantics rather than a stale
                // cached y coordinate. Only the very first top-of-screen paint
                // may fall back to cursor geometry before semantic rows arrive.
                let cursor_row = rc.cursor.state.pos.row.0;
                let semantic_live_anchor = if cursor_row >= 0 {
                    let cursor_index = cursor_row as usize;
                    rc.visible_rows
                        .iter()
                        .enumerate()
                        .rev()
                        .find_map(|(row_index, row)| {
                            if row_index == 0
                                || row_index > cursor_index
                                || row.semantic_prompt
                                    != rio_backend::crosswords::grid::row::SemanticPrompt::PromptContinuation
                            {
                                return None;
                            }
                            let prompt_row = &rc.visible_rows[row_index - 1];
                            if prompt_row.semantic_prompt
                                != rio_backend::crosswords::grid::row::SemanticPrompt::Prompt
                            {
                                return None;
                            }
                            let blank = prompt_row.inner.iter().all(|sq| {
                                sq.is_bg_only() || matches!(sq.c(), '\\0' | ' ')
                            });
                            if !blank { return None; }
                            let prompt_index = row_index - 1;
                            Some(crate::automexia::ui::PromptAnchor {
                                key: first_absolute_row.saturating_add(prompt_index as u64),
                                x: origin_x,
                                y: origin_y + prompt_index as f32 * cell_height,
                                width: grid_width,
                                height: cell_height,
                            })
                        })
                } else {
                    None
                };

                let live_anchor = if rc.shell_integration
                    && rc.shell_prompt_active
                    && rc.display_offset == 0
                {
                    semantic_live_anchor.or_else(|| {
                        // First prompt after startup/clear: the prompt consists
                        // of exactly the blank context row plus the editable row.
                        // Do not use this fallback deeper in the screen because a
                        // long wrapped command can legitimately scroll its context
                        // row out of view.
                        if cursor_row == 1 {
                            Some(crate::automexia::ui::PromptAnchor {
                                key: first_absolute_row,
                                x: origin_x,
                                y: origin_y,
                                width: grid_width,
                                height: cell_height,
                            })
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };

                (
                    crate::automexia::api::SessionFacts {
                        session_id: context.route_id,
                        cwd: rc.current_directory.clone(),
                        title: rc.terminal_title.clone(),
                        distro: rc.shell_distro.clone(),
                        os_version: rc.shell_os_version.clone(),
                        shell_integration: rc.shell_integration,
                        shell_pid: context.shell_pid,
                    },
                    rc.shell_prompt_active,
                    historical_anchors,
                    live_anchor,
                )
            };
            let refresh_pending = self.devops_status.render_prompt_rows(
                sugarloaf,
                self.named_colors,
                &session,
                prompt_active,
                &historical_anchors,
                live_anchor,
            );
            if refresh_pending {
                context_manager.schedule_render_on_route(100);
            }
        }

'''
    current_prompt_hud = (
        "let semantic_live_anchor =" in text
        and "SemanticPrompt::PromptContinuation" in text
        and "historical_anchors.last().copied()" not in text
    )
    if not current_prompt_hud:
        text = text[:search_at] + prompt_hud + text[command_at:]
    return text


def patch_renderer_utils(text: str) -> str:
    """Restore stock navigation padding; prompt context now occupies terminal rows."""
    stock = """#[inline]
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
"""
    legacy = """#[inline]
pub fn extension_status_y_from_config(
    navigation: &Navigation,
    #[allow(unused)] num_tabs: usize,
    #[allow(unused)] macos_use_unified_titlebar: bool,
) -> f32 {
    // The extension status lane starts immediately after navigation chrome.
    if navigation.is_enabled() {
        #[cfg(not(target_os = "macos"))]
        if navigation.hide_if_single && num_tabs <= 1 {
            return constants::PADDING_Y;
        }

        use crate::renderer::island::ISLAND_HEIGHT;
        return ISLAND_HEIGHT;
    }

    #[cfg(target_os = "macos")]
    {
        use rio_backend::config::navigation::NavigationMode;
        if navigation.mode == NavigationMode::NativeTab {
            return if macos_use_unified_titlebar {
                constants::ADDITIONAL_PADDING_Y_ON_UNIFIED_TITLEBAR
            } else {
                0.0
            };
        }
    }

    constants::PADDING_Y
}

#[inline]
pub fn padding_top_from_config(
    navigation: &Navigation,
    padding_y_top: f32,
    num_tabs: usize,
    macos_use_unified_titlebar: bool,
) -> f32 {
    extension_status_y_from_config(
        navigation,
        num_tabs,
        macos_use_unified_titlebar,
    ) + crate::automexia::ui::EXTENSION_STATUS_BAR_HEIGHT
        + padding_y_top
}
"""
    if legacy in text:
        return text.replace(legacy, stock, 1)
    if stock in text:
        return text
    raise RuntimeError("Automexia semantic prompt geometry: renderer padding marker missing")


def patch_palette(text: str) -> str:
    text = text.replace("use crate::extensions::manager::ExtensionMarketItem;\n", "use crate::automexia::marketplace::MarketItem;\n")
    text = text.replace("ExtensionMarketItem", "MarketItem")
    text = replace_once(
        text,
        "use crate::renderer::scrollbar;\n",
        "use crate::automexia::marketplace::MarketItem;\nuse crate::renderer::scrollbar;\n",
        "palette market import",
    )

    platform_labels = '''\n#[cfg(target_os = "macos")]\nconst SHORTCUT_NEW_TAB: &str = "Cmd+T";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_NEW_TAB: &str = "Ctrl+Shift+T";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_NEW_TAB: &str = "Ctrl+Shift+T";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_CLOSE: &str = "Cmd+W";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_CLOSE: &str = "Ctrl+Shift+W";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_CLOSE: &str = "Ctrl+Shift+W";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_SPLIT_RIGHT: &str = "Cmd+D";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_SPLIT_RIGHT: &str = "Ctrl+Shift+R";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_SPLIT_RIGHT: &str = "Ctrl+Shift+R";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_SPLIT_DOWN: &str = "Cmd+Shift+D";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_SPLIT_DOWN: &str = "Ctrl+Shift+D";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_SPLIT_DOWN: &str = "Ctrl+Shift+D";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_SETTINGS: &str = "Cmd+,";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_SETTINGS: &str = "Ctrl+,";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_SETTINGS: &str = "Ctrl+Shift+,";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_NEW_WINDOW: &str = "Cmd+N";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_NEW_WINDOW: &str = "Ctrl+Shift+N";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_NEW_WINDOW: &str = "Ctrl+Shift+N";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_COPY: &str = "Cmd+C";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_COPY: &str = "Ctrl+Shift+C";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_COPY: &str = "Ctrl+Shift+C";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_PASTE: &str = "Cmd+V";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_PASTE: &str = "Ctrl+Shift+V";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_PASTE: &str = "Ctrl+Shift+V";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_SEARCH: &str = "Cmd+F";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_SEARCH: &str = "Ctrl+Shift+F";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_SEARCH: &str = "Ctrl+Shift+F";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_FONT_UP: &str = "Cmd++";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_FONT_UP: &str = "Ctrl++";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_FONT_UP: &str = "Ctrl++";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_FONT_DOWN: &str = "Cmd+-";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_FONT_DOWN: &str = "Ctrl+-";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_FONT_DOWN: &str = "Ctrl+-";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_FONT_RESET: &str = "Cmd+0";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_FONT_RESET: &str = "Ctrl+0";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_FONT_RESET: &str = "Ctrl+0";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_VI_MODE: &str = "Alt+Shift+Space";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_VI_MODE: &str = "Ctrl+Shift+Space";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_VI_MODE: &str = "Alt+Shift+Space";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_FULLSCREEN: &str = "Ctrl+Cmd+F";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_FULLSCREEN: &str = "F11";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_FULLSCREEN: &str = "";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_APPEARANCE: &str = "";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_APPEARANCE: &str = "Alt+Shift+T";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_APPEARANCE: &str = "";\n#[cfg(target_os = "macos")]\nconst SHORTCUT_CLEAR_HISTORY: &str = "Cmd+K";\n#[cfg(target_os = "windows")]\nconst SHORTCUT_CLEAR_HISTORY: &str = "Ctrl+Shift+K";\n#[cfg(not(any(target_os = "macos", target_os = "windows")))]\nconst SHORTCUT_CLEAR_HISTORY: &str = "";\n'''
    text = replace_once(
        text,
        "const ORDER: u8 = 20;\n",
        "const ORDER: u8 = 20;\n" + platform_labels,
        "palette platform labels",
    )
    replacements = {
        'shortcut: "Cmd+T",': 'shortcut: SHORTCUT_NEW_TAB,',
        'shortcut: "Cmd+W",': 'shortcut: SHORTCUT_CLOSE,',
        'shortcut: "Cmd+D",': 'shortcut: SHORTCUT_SPLIT_RIGHT,',
        'shortcut: "Cmd+Shift+D",': 'shortcut: SHORTCUT_SPLIT_DOWN,',
        'shortcut: "Cmd+,",': 'shortcut: SHORTCUT_SETTINGS,',
        'shortcut: "Cmd+N",': 'shortcut: SHORTCUT_NEW_WINDOW,',
        'shortcut: "Cmd+C",': 'shortcut: SHORTCUT_COPY,',
        'shortcut: "Cmd+V",': 'shortcut: SHORTCUT_PASTE,',
        'shortcut: "Cmd+F",': 'shortcut: SHORTCUT_SEARCH,',
        'shortcut: "Cmd++",': 'shortcut: SHORTCUT_FONT_UP,',
        'shortcut: "Cmd+-",': 'shortcut: SHORTCUT_FONT_DOWN,',
        'shortcut: "Cmd+0",': 'shortcut: SHORTCUT_FONT_RESET,',
    }
    for old, new in replacements.items():
        text = replace_once(text, old, new, f"palette label {old}")

    for title, shortcut in [
        ("Toggle Vi Mode", "SHORTCUT_VI_MODE"),
        ("Toggle Fullscreen", "SHORTCUT_FULLSCREEN"),
        ("Toggle Appearance Theme", "SHORTCUT_APPEARANCE"),
        ("Clear History", "SHORTCUT_CLEAR_HISTORY"),
    ]:
        text = replace_once(
            text,
            f'title: "{title}",\n        shortcut: "",',
            f'title: "{title}",\n        shortcut: {shortcut},',
            f"palette label {title}",
        )

    text = replace_once(
        text,
        "    CloseCurrentSplitOrTab,\n    /// Browse the family names",
        "    CloseCurrentSplitOrTab,\n    OpenMarket,\n    /// Browse the family names",
        "palette OpenMarket action",
    )
    text = replace_once(
        text,
        '''    Command {\n        title: "List Fonts",\n        shortcut: "",\n        action: PaletteAction::ListFonts,\n    },\n''',
        '''    Command {\n        title: "/market · Browse extensions",\n        shortcut: "",\n        action: PaletteAction::OpenMarket,\n    },\n    Command {\n        title: "List Fonts",\n        shortcut: "",\n        action: PaletteAction::ListFonts,\n    },\n''',
        "palette market command",
    )
    text = replace_once(
        text,
        "enum PaletteMode {\n    Commands,\n    Fonts(Vec<String>),\n}",
        "enum PaletteMode {\n    Commands,\n    Fonts(Vec<String>),\n    Market(Vec<MarketItem>),\n}",
        "palette market mode",
    )
    text = replace_once(
        text,
        '''    Font {\n        family: &'a str,\n    },\n}''',
        '''    Font {\n        family: &'a str,\n    },\n    Market {\n        id: &'a str,\n        name: &'a str,\n        installed: bool,\n    },\n}''',
        "palette market row",
    )
    text = replace_once(
        text,
        '''            PaletteRow::Command { title, .. } => title,\n            PaletteRow::Font { family } => family,\n''',
        '''            PaletteRow::Command { title, .. } => title,\n            PaletteRow::Font { family } => family,\n            PaletteRow::Market { name, .. } => name,\n''',
        "palette title market",
    )
    text = replace_once(
        text,
        '''            PaletteRow::Command { shortcut, .. } => shortcut,\n            PaletteRow::Font { .. } => "",\n''',
        '''            PaletteRow::Command { shortcut, .. } => shortcut,\n            PaletteRow::Font { .. } => "",\n            PaletteRow::Market { installed: true, .. } => "Remove",\n            PaletteRow::Market { installed: false, .. } => "Install",\n''',
        "palette shortcut market",
    )
    text = replace_once(
        text,
        '''            PaletteRow::Command { action, .. } => Some(action),\n            PaletteRow::Font { .. } => None,\n''',
        '''            PaletteRow::Command { action, .. } => Some(action),\n            PaletteRow::Font { .. } | PaletteRow::Market { .. } => None,\n''',
        "palette action market",
    )

    enter_marker = '''    pub fn enter_fonts_mode(&mut self, fonts: Vec<String>) {\n        self.mode = PaletteMode::Fonts(fonts);\n        self.query.clear();\n        self.selected_index = 0;\n        self.scroll_offset = 0;\n        self.caret_blink_start = Instant::now();\n        self.last_scroll_time = None;\n    }\n'''
    enter_new = enter_marker + '''\n    pub fn enter_market_mode(&mut self, items: Vec<MarketItem>) {\n        self.mode = PaletteMode::Market(items);\n        self.query.clear();\n        self.selected_index = 0;\n        self.scroll_offset = 0;\n        self.caret_blink_start = Instant::now();\n        self.last_scroll_time = None;\n    }\n'''
    text = replace_once(text, enter_marker, enter_new, "palette enter market")

    selected_font_marker = '''    pub fn get_selected_font(&self) -> Option<String> {\n        self.filtered_rows()\n            .get(self.selected_index)\n            .and_then(|(_, row)| match row {\n                PaletteRow::Font { family } => Some((*family).to_owned()),\n                PaletteRow::Command { .. } => None,\n            })\n    }\n'''
    selected_font_new = '''    pub fn get_selected_font(&self) -> Option<String> {\n        self.filtered_rows()\n            .get(self.selected_index)\n            .and_then(|(_, row)| match row {\n                PaletteRow::Font { family } => Some((*family).to_owned()),\n                PaletteRow::Command { .. } | PaletteRow::Market { .. } => None,\n            })\n    }\n\n    pub fn get_selected_market_id(&self) -> Option<String> {\n        self.filtered_rows()\n            .get(self.selected_index)\n            .and_then(|(_, row)| match row {\n                PaletteRow::Market { id, .. } => Some((*id).to_owned()),\n                PaletteRow::Command { .. } | PaletteRow::Font { .. } => None,\n            })\n    }\n'''
    text = replace_once(text, selected_font_marker, selected_font_new, "palette selected market")

    market_match = '''            PaletteMode::Fonts(fonts) => fonts\n                .iter()\n                .filter_map(|family| {\n                    let score = fuzzy_score(&self.query, family)?;\n                    Some((score, PaletteRow::Font { family }))\n                })\n                .collect(),\n'''
    market_new = market_match + '''            PaletteMode::Market(items) => items\n                .iter()\n                .filter_map(|item| {\n                    let score = [item.name.as_str(), item.id.as_str(), item.description.as_str()]\n                        .into_iter()\n                        .filter_map(|candidate| fuzzy_score(&self.query, candidate))\n                        .max()?;\n                    Some((\n                        score,\n                        PaletteRow::Market {\n                            id: &item.id,\n                            name: &item.name,\n                            installed: item.installed,\n                        },\n                    ))\n                })\n                .collect(),\n'''
    market_arm_count = text.count("PaletteMode::Market(items) => items")
    if market_arm_count == 0:
        text = replace_once(text, market_match, market_new, "palette market filtering")
    elif market_arm_count != 1:
        raise RuntimeError(
            f"palette market filtering: expected one existing market arm, found {market_arm_count}"
        )
    text = replace_once(
        text,
        '''            PaletteMode::Commands => "Type a command...",\n            PaletteMode::Fonts(_) => "Type a font name...",\n''',
        '''            PaletteMode::Commands => "Type a command...",\n            PaletteMode::Fonts(_) => "Type a font name...",\n            PaletteMode::Market(_) => "Search extensions...",\n''',
        "palette market placeholder",
    )
    return text


def patch_router(text: str) -> str:
    # Upgrade already-patched v0.2 routers directly to the Automexia runtime.
    text = text.replace("crate::extensions::manager::toggle", "crate::automexia::runtime::toggle")
    text = text.replace("crate::extensions::manager::market_items", "crate::automexia::runtime::market_items")
    text = replace_once(
        text,
        '''                        let selected_font = self\n                            .window\n                            .screen\n                            .renderer\n                            .command_palette\n                            .get_selected_font();\n                        let selected_action = self\n''',
        '''                        let selected_font = self\n                            .window\n                            .screen\n                            .renderer\n                            .command_palette\n                            .get_selected_font();\n                        let selected_market_id = self\n                            .window\n                            .screen\n                            .renderer\n                            .command_palette\n                            .get_selected_market_id();\n                        let selected_action = self\n''',
        "router selected market",
    )
    marker = '''                        match selected_action {\n                            // `ListFonts` stays inside the palette —\n'''
    insertion = '''                        if let Some(extension_id) = selected_market_id {\n                            match crate::automexia::runtime::toggle(&extension_id) {\n                                Ok(_) => {\n                                    // Manager generation changes atomically. The requested redraw\n                                    // lets Renderer::run synchronize the state and force full damage\n                                    // for every visible split before row emission.\n                                    let items = crate::automexia::runtime::market_items();\n                                    self.window\n                                        .screen\n                                        .renderer\n                                        .command_palette\n                                        .enter_market_mode(items);\n                                }\n                                Err(error) => tracing::warn!(\n                                    "extension activation change failed for {}: {}",\n                                    extension_id,\n                                    error\n                                ),\n                            }\n                            self.request_overlay_redraw();\n                            return true;\n                        }\n\n                        match selected_action {\n                            Some(PaletteAction::OpenMarket) => {\n                                let items = crate::automexia::runtime::market_items();\n                                self.window\n                                    .screen\n                                    .renderer\n                                    .command_palette\n                                    .enter_market_mode(items);\n                            }\n                            // `ListFonts` stays inside the palette —\n'''
    return replace_once(text, marker, insertion, "router market handling")


def patch_screen(text: str) -> str:
    # Renderer::new applies the unified palette. Preserve that same palette when
    # Screen keeps an existing tab island across config reloads.
    stock_island = """        if let Some(mut island) = old_island {
            island.update_colors(config.colors.tabs, config.colors.tabs_active);
            island.max_tab_width = config.navigation.max_tab_width;
            self.renderer.island = Some(island);
        }
"""
    themed_island = """        if let Some(mut island) = old_island {
            let automexia_colors = crate::automexia::theme::effective_colors(config.colors);
            island.update_colors(automexia_colors.tabs, automexia_colors.tabs_active);
            island.max_tab_width = config.navigation.max_tab_width;
            self.renderer.island = Some(island);
        }
"""
    if themed_island not in text and stock_island in text:
        text = text.replace(stock_island, themed_island, 1)

    marker = '''            PaletteAction::ListFonts => {\n                // Handled in the router: switches the palette into fonts\n'''
    insertion = '''            PaletteAction::OpenMarket => {\n                // Handled by the router because it changes palette mode.\n            }\n            PaletteAction::ListFonts => {\n                // Handled in the router: switches the palette into fonts\n'''
    return replace_once(text, marker, insertion, "screen market action")


def patch_grid_emit(text: str) -> str:
    # Upgrade v0.2 semantic-highlight call sites; the built-in implementation
    # is now hidden behind the Automexia runtime/API boundary.
    text = text.replace(
        "crate::extensions::devops::classify_row_text",
        "crate::automexia::runtime::classify_row_text",
    )
    text = text.replace(
        "crate::extensions::devops::SemanticSeverity",
        "crate::automexia::api::SemanticSeverity",
    )
    helper_marker = "use rio_backend::sugarloaf::font::FontLibrary;\n"
    helper = '''fn semantic_row_fg(
    row: &Row<Square>,
    cols: usize,
    renderer: &Renderer,
    scratch: &mut String,
) -> Option<[u8; 4]> {
    if !renderer.devops_enabled {
        return None;
    }
    scratch.clear();
    if scratch.capacity() < cols {
        scratch.reserve(cols);
    }
    for square in row.inner.iter().take(cols) {
        if square.content_tag() != ContentTag::Codepoint {
            continue;
        }
        let character = square.c();
        if character != '\0' && !character.is_control() {
            scratch.push(character);
        }
    }
    match crate::automexia::runtime::classify_row_text(scratch)? {
        crate::automexia::api::SemanticSeverity::Error =>
            Some(normalized_to_u8(renderer.named_colors.red)),
        crate::automexia::api::SemanticSeverity::Warning =>
            Some(normalized_to_u8(renderer.named_colors.yellow)),
        crate::automexia::api::SemanticSeverity::Success =>
            Some(normalized_to_u8(renderer.named_colors.green)),
        crate::automexia::api::SemanticSeverity::Info =>
            Some(normalized_to_u8(renderer.named_colors.cyan)),
        crate::automexia::api::SemanticSeverity::Debug =>
            Some(normalized_to_u8(renderer.named_colors.blue)),
    }
}

#[inline]
fn semantic_or_cell_fg(
    semantic: Option<[u8; 4]>,
    sq: Square,
    style: Style,
    renderer: &Renderer,
    term_colors: &TermColors,
) -> [u8; 4] {
    // Preserve colors explicitly produced by applications (kubecolor,
    // Terraform, TUIs, etc.). Semantic highlighting only improves plain
    // default-foreground output.
    let can_override = !style.flags.contains(StyleFlags::INVERSE)
        && matches!(
            style.fg,
            AnsiColor::Named(NamedColor::Foreground)
                | AnsiColor::Named(NamedColor::White)
                | AnsiColor::Named(NamedColor::LightWhite)
        );
    if can_override {
        if let Some(color) = semantic {
            return color;
        }
    }
    cell_fg(sq, style, renderer, term_colors)
}

use rio_backend::sugarloaf::font::FontLibrary;
'''
    # Fresh audited Rio has only the FontLibrary marker. v0.2/v0.3 checkouts
    # may already contain an older Automexia helper; replace that entire helper
    # in-place so upgrades do not define duplicate semantic_row_fg functions.
    existing_helper = re.compile(
        r"fn semantic_row_fg\(.*?use rio_backend::sugarloaf::font::FontLibrary;\n",
        flags=re.DOTALL,
    )
    helper_matches = list(existing_helper.finditer(text))
    if helper_matches:
        if len(helper_matches) != 1:
            raise RuntimeError(
                f"semantic helper upgrade: expected one existing helper, found {len(helper_matches)}"
            )
        match = helper_matches[0]
        existing = match.group(0)
        helper_is_current = all(
            marker in existing
            for marker in (
                "SemanticSeverity::Info",
                "SemanticSeverity::Debug",
                "fn semantic_or_cell_fg(",
            )
        )
        if not helper_is_current:
            text = text[: match.start()] + helper + text[match.end() :]
    else:
        text = replace_once(text, helper_marker, helper, "semantic helper")

    text = replace_once(
        text,
        "    run_cell_columns: Vec<u16>,\n",
        "    run_cell_columns: Vec<u16>,\n    /// Reused DevOps row-classification buffer; no per-row String allocation after warm-up.\n    semantic_text_scratch: String,\n",
        "semantic scratch field",
    )
    text = replace_once(
        text,
        "            run_cell_columns: Vec::new(),\n",
        "            run_cell_columns: Vec::new(),\n            semantic_text_scratch: String::new(),\n",
        "semantic scratch init",
    )
    text = replace_once(
        text,
        "    let needs_per_cell_check = has_sel || has_color_hints;\n",
        "    let semantic_fg = semantic_row_fg(row, cols, renderer, &mut rasterizer.semantic_text_scratch);\n    let needs_per_cell_check = has_sel || has_color_hints || semantic_fg.is_some();\n",
        "semantic setup",
    )

    def patch_last_fg_in_region(
        source: str,
        start_marker: str,
        end_marker: str,
        cell_pattern: str,
        replacement: str,
        label: str,
    ) -> str:
        """Patch the fallback foreground inside one audited rendering region.

        The region markers are semantic comments/statements that are stable across
        rustfmt. Within a region, the final plain `cell_fg(...)` is the fallback
        reached after selection/search handling. The earlier one (when present)
        is the no-selection fast path and intentionally stays unchanged because
        `semantic_fg.is_some()` now disables that fast path.
        """
        start_count = source.count(start_marker)
        if start_count != 1:
            raise RuntimeError(f"{label}: expected one start marker, found {start_count}")
        region_start = source.index(start_marker)
        region_end = source.find(end_marker, region_start + len(start_marker))
        if region_end < 0:
            raise RuntimeError(f"{label}: end marker not found")

        region = source[region_start:region_end]
        semantic_count = region.count("semantic_or_cell_fg(")
        if semantic_count == 1:
            return source
        if semantic_count > 1:
            raise RuntimeError(f"{label}: expected at most one semantic fallback, found {semantic_count}")

        matches = list(re.finditer(cell_pattern, region, flags=re.DOTALL))
        if not matches:
            raise RuntimeError(f"{label}: no compatible plain foreground fallback found")
        match = matches[-1]
        region = region[: match.start()] + replacement + region[match.end() :]
        if region.count("semantic_or_cell_fg(") != 1:
            raise RuntimeError(f"{label}: semantic fallback invariant was not established")
        return source[:region_start] + region + source[region_end:]

    # Patch the two one-cell short-circuit paths independently. This accepts a
    # mixed source state (one already patched, one stock) and tolerates comments
    # or rustfmt whitespace changing around the fallback expression.
    sq_fg_pattern = r"cell_fg\(\s*sq\s*,\s*style\s*,\s*renderer\s*,\s*term_colors\s*\)"
    sq_semantic = "semantic_or_cell_fg(semantic_fg, sq, style, renderer, term_colors)"
    text = patch_last_fg_in_region(
        text,
        "        // Glyph Protocol short-circuit:",
        "        // Built-in drawable sprite short-circuit:",
        sq_fg_pattern,
        sq_semantic,
        "semantic custom-glyph path",
    )
    text = patch_last_fg_in_region(
        text,
        "        // Built-in drawable sprite short-circuit:",
        "        let run_start = x;",
        sq_fg_pattern,
        sq_semantic,
        "semantic drawable-glyph path",
    )

    # Patch the regular shaped-glyph fallback in the same way. Selection/search
    # branches remain ahead of this fallback, preserving their visual priority.
    src_fg_pattern = (
        r"cell_fg\(\s*src_sq\s*,\s*src_style\s*,\s*renderer\s*,\s*term_colors\s*\)"
    )
    src_semantic = (
        "semantic_or_cell_fg(semantic_fg, src_sq, src_style, renderer, term_colors)"
    )
    text = patch_last_fg_in_region(
        text,
        "            // Pull fg from the cluster's first cell.",
        "            fg_scratch.push(CellText {",
        src_fg_pattern,
        src_semantic,
        "semantic regular glyph path",
    )
    return text


WINDOWS_BINDINGS = r'''// Windows
#[cfg(all(target_os = "windows", not(test)))]
pub fn platform_key_bindings(
    use_navigation_key_bindings: bool,
    use_splits: bool,
    _: ConfigKeyboard,
) -> Vec<KeyBinding> {
    // AUTOMEXIA WINDOWS DEFAULTS v2.3 — application shortcuts follow
    // Windows/browser conventions while readline/editor control keys remain PTY input.
    let mut key_bindings = bindings!(
        KeyBinding;
        "v", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::Paste;
        "c", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::Copy;
        "c", ModifiersState::CONTROL | ModifiersState::SHIFT, +BindingMode::VI; Action::ClearSelection;
        "a", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI, ~BindingMode::SEARCH; Action::SelectAll;
        "k", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::VI; Action::ClearHistory;
        Key::Named(Insert), ModifiersState::SHIFT, ~BindingMode::VI; Action::PasteSelection;
        "0", ModifiersState::CONTROL; Action::ResetFontSize;
        "=", ModifiersState::CONTROL; Action::IncreaseFontSize;
        "+", ModifiersState::CONTROL; Action::IncreaseFontSize;
        "-", ModifiersState::CONTROL; Action::DecreaseFontSize;
        Key::Named(Enter), ModifiersState::ALT; Action::ToggleFullscreen;
        Key::Named(F11); Action::ToggleFullscreen;
        "n", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::WindowCreateNew;
        ",", ModifiersState::CONTROL; Action::ConfigEditor;
        "p", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::OpenCommandPalette;
        Key::Named(Space), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::ToggleViMode;
        Key::Named(Space), ModifiersState::CONTROL | ModifiersState::ALT; Action::ToggleQuake;
        "t", ModifiersState::ALT | ModifiersState::SHIFT; Action::ToggleAppearanceTheme;
        Key::Named(Backspace), ModifiersState::CONTROL, ~BindingMode::VI; Action::Esc("\u{0017}".into());

        // Search. Backward search remains Shift+Enter inside search mode.
        "f", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::SEARCH; Action::SearchForward;
        "c", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchCancel;
        "u", ModifiersState::CONTROL, +BindingMode::SEARCH; SearchAction::SearchClear;
        "w", ModifiersState::CONTROL,  +BindingMode::SEARCH; SearchAction::SearchDeleteWord;
        "p", ModifiersState::CONTROL,  +BindingMode::SEARCH; SearchAction::SearchHistoryPrevious;
        "n", ModifiersState::CONTROL,  +BindingMode::SEARCH; SearchAction::SearchHistoryNext;
        Key::Named(ArrowUp), +BindingMode::SEARCH; SearchAction::SearchHistoryPrevious;
        Key::Named(ArrowDown), +BindingMode::SEARCH; SearchAction::SearchHistoryNext;
    );

    if use_navigation_key_bindings {
        key_bindings.extend(bindings!(
            KeyBinding;
            "t", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::TabCreateNew;
            "w", ModifiersState::CONTROL | ModifiersState::SHIFT; Action::CloseCurrentSplitOrTab;
            Key::Named(Tab), ModifiersState::CONTROL; Action::SelectNextTab;
            Key::Named(Tab), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::SelectPrevTab;
            Key::Named(PageUp), ModifiersState::CONTROL; Action::SelectPrevTab;
            Key::Named(PageDown), ModifiersState::CONTROL; Action::SelectNextTab;
            Key::Named(PageUp), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::MoveCurrentTabToPrev;
            Key::Named(PageDown), ModifiersState::CONTROL | ModifiersState::SHIFT; Action::MoveCurrentTabToNext;
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
            "r", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitRight;
            "d", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitDown;
            Key::Named(F6), ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SelectNextSplit;
            Key::Named(F6), ModifiersState::SHIFT, ~BindingMode::ALT_SCREEN, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SelectPrevSplit;
            Key::Named(ArrowUp), ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerUp;
            Key::Named(ArrowDown), ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerDown;
            Key::Named(ArrowLeft), ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerLeft;
            Key::Named(ArrowRight), ModifiersState::ALT | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::MoveDividerRight;
        ));
    }

    // Hint bindings are added separately in Screen::new() based on config.
    key_bindings
}
'''


def patch_bindings(text: str) -> str:
    if "AUTOMEXIA WINDOWS DEFAULTS v2.3" in text:
        return text
    pattern = r'''// Windows\n#\[cfg\(all\(target_os = "windows", not\(test\)\)\)\]\npub fn platform_key_bindings\(.*?\n}\n\n#\[cfg\(test\)\]'''
    match = re.search(pattern, text, flags=re.DOTALL)
    if not match:
        raise RuntimeError("Windows bindings function: expected one source block")
    replacement = WINDOWS_BINDINGS + '\n#[cfg(test)]'
    return text[: match.start()] + replacement + text[match.end() :]


def patch_windows_util_warnings(text: str) -> str:
    """Normalize bare Win32 function-pointer parameter names to Rust style.

    Parameter names in a bare ``extern \"system\" fn`` type are documentation-only
    and are not part of the Windows ABI. Restrict the rename to the audited dynamic
    User32 function-pointer block so call sites and ordinary local variables are
    untouched. The transformation also repairs partially-renamed source states.
    """
    block_pattern = re.compile(
        r'pub type AdjustWindowRectExForDpi\s*=.*?'
        r'pub type GetPointerPenInfo\s*=\s*unsafe extern "system" fn\(.*?\)\s*->\s*BOOL;',
        flags=re.DOTALL,
    )
    matches = list(block_pattern.finditer(text))
    if len(matches) != 1:
        raise RuntimeError(
            f"Windows FFI warning cleanup: expected exactly one function-pointer block, found {len(matches)}"
        )

    match = matches[0]
    block = match.group(0)
    renames = {
        "dwStyle": "dw_style",
        "bMenu": "b_menu",
        "dwExStyle": "dw_ex_style",
        "pointerId": "pointer_id",
        "entriesCount": "entries_count",
        "pointerCount": "pointer_count",
        "pointerInfo": "pointer_info",
        "pointerDeviceRect": "pointer_device_rect",
        "displayRect": "display_rect",
        "touchInfo": "touch_info",
        "pointId": "point_id",
        "penInfo": "pen_info",
    }
    for old, new in renames.items():
        block = re.sub(rf"\b{re.escape(old)}\b", new, block)

    leftovers = [old for old in renames if re.search(rf"\b{re.escape(old)}\b", block)]
    if leftovers:
        raise RuntimeError(
            "Windows FFI warning cleanup left non-snake-case parameters: " + ", ".join(leftovers)
        )
    required = [new for new in renames.values() if new not in block]
    if required:
        raise RuntimeError(
            "Windows FFI warning cleanup could not verify renamed parameters: " + ", ".join(required)
        )
    return text[: match.start()] + block + text[match.end() :]


def _replace_irrefutable_wgpu_block(text: str, variable: str, expected: int) -> str:
    old_pattern = re.compile(
        rf'(?m)^([ \t]*)if let ImageTexture::Wgpu \{{ view, \.\. \}} = &{re.escape(variable)}\.gpu \{{$'
    )
    new_marker = f"let ImageTexture::Wgpu {{ view, .. }} = &{variable}.gpu;"
    old_count = len(old_pattern.findall(text))
    new_count = text.count(new_marker)
    if old_count == 0:
        if new_count != expected:
            raise RuntimeError(
                f"Sugarloaf {variable} Wgpu warning cleanup: expected {expected} cleaned marker(s), found {new_count}"
            )
        return text
    if old_count + new_count != expected:
        raise RuntimeError(
            f"Sugarloaf {variable} Wgpu warning cleanup: expected {expected} logical site(s), "
            f"found {old_count} old + {new_count} cleaned"
        )

    def repl(match: re.Match[str]) -> str:
        indent = match.group(1)
        # Keep the original closing brace by turning the `if let` into an
        # unconditional scoped block containing an irrefutable `let`.
        return (
            f"{indent}{{\n"
            f"{indent}    let ImageTexture::Wgpu {{ view, .. }} = &{variable}.gpu;"
        )

    return old_pattern.sub(repl, text)


def patch_sugarloaf_warnings(text: str) -> str:
    # On the audited WGPU renderer ImageTexture has one reachable variant in
    # these paths. A scoped `let` preserves lifetime/brace structure while
    # removing Rust's `irrefutable_let_patterns` warning.
    text = _replace_irrefutable_wgpu_block(text, "bg_tex", 1)
    text = _replace_irrefutable_wgpu_block(text, "img", 2)
    return text


def patch_title_warnings(text: str) -> str:
    # `context` and `path` are consumed only by #[cfg(unix)] code. Explicitly
    # mark them as used on non-Unix targets instead of renaming public/internal
    # variables or globally suppressing the unused lint.
    context_old = '''    context: &Context<T>,\n) -> Option<ContextTitleExtra> {\n    #[cfg(unix)]\n'''
    context_new = '''    context: &Context<T>,\n) -> Option<ContextTitleExtra> {\n    #[cfg(not(unix))]\n    let _ = context;\n\n    #[cfg(unix)]\n'''
    text = replace_once(
        text,
        context_old,
        context_new,
        "Windows title context unused-variable cleanup",
    )

    path_old = '''    let path = Path::new(absolute);\n\n    // Replace home prefix with ~\n'''
    path_new = '''    let path = Path::new(absolute);\n    #[cfg(not(unix))]\n    let _ = path;\n\n    // Replace home prefix with ~\n'''
    return replace_once(
        text,
        path_old,
        path_new,
        "Windows title path unused-variable cleanup",
    )


PATCHERS = {
    "frontends/rioterm/Cargo.toml": patch_cargo,
    "frontends/rioterm/src/main.rs": patch_main,
    "frontends/rioterm/src/context/renderable.rs": patch_renderable,
    "frontends/rioterm/src/context/mod.rs": patch_context_session,
    "frontends/rioterm/src/renderer/mod.rs": patch_renderer,
    "frontends/rioterm/src/renderer/utils.rs": patch_renderer_utils,
    "frontends/rioterm/src/renderer/command_palette.rs": patch_palette,
    "frontends/rioterm/src/router/mod.rs": patch_router,
    "frontends/rioterm/src/screen/mod.rs": patch_screen,
    "frontends/rioterm/src/grid_emit.rs": patch_grid_emit,
    "frontends/rioterm/src/bindings/mod.rs": patch_bindings,
    "rio-window/src/platform_impl/windows/util.rs": patch_windows_util_warnings,
    "sugarloaf/src/renderer/mod.rs": patch_sugarloaf_warnings,
    "frontends/rioterm/src/context/title.rs": patch_title_warnings,
}


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def main() -> None:
    parser = argparse.ArgumentParser(description="Apply Automexia Terminal rustfmt-normalized v0.3.13 integration to Rio")
    parser.add_argument("project_root", type=Path)
    parser.add_argument("--allow-dirty", action="store_true")
    args = parser.parse_args()
    root = args.project_root.resolve()

    if not (root / "Cargo.toml").is_file() or not (root / "frontends/rioterm/src/main.rs").is_file():
        raise SystemExit(f"Not a Rio source checkout: {root}")
    if (root / ".git").exists():
        if not args.allow_dirty:
            status = git(root, "status", "--porcelain")
            if status:
                raise SystemExit("Rio checkout has uncommitted changes; commit/stash them before applying Automexia.")

        target_known = subprocess.run(
            ["git", "-C", str(root), "cat-file", "-e", f"{UPSTREAM_TARGET}^{{commit}}"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode == 0
        if not target_known:
            raise SystemExit(
                "The audited Rio base is not present in this checkout. Run BOOTSTRAP-WINDOWS.ps1 first."
            )
        compatible = subprocess.run(
            ["git", "-C", str(root), "merge-base", "--is-ancestor", UPSTREAM_TARGET, "HEAD"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode == 0
        if not compatible:
            raise SystemExit(
                "This checkout is older than or diverged from the audited Rio base. "
                "Use BOOTSTRAP-WINDOWS.ps1 instead of forcing the patch."
            )

    stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    # Keep backups beside the checkout instead of inside it. An in-tree
    # `.backups` directory makes `git status` dirty and turns a recoverable
    # patch failure into a second bootstrap failure on the next run.
    backup_parent = root.parent / f"{root.name}.backups"
    backup = backup_parent / f"automexia-terminal-v0.3.13-{stamp}"
    backup.mkdir(parents=True, exist_ok=False)

    package_root = Path(__file__).resolve().parent
    overlay_root = package_root / "overlay"

    paths_to_backup = set(PATCHED_FILES)
    paths_to_backup.update(OBSOLETE_FILES)
    overlay_files: list[tuple[Path, Path, str]] = []
    for source in overlay_root.rglob("*"):
        if not source.is_file():
            continue
        relative_path = source.relative_to(overlay_root)
        relative = str(relative_path).replace("\\", "/")
        paths_to_backup.add(relative)
        overlay_files.append((source, root / relative_path, relative))

    # Capture the exact pre-apply state before doing any source transformation.
    # This makes both semantic-patch failures and write failures recoverable.
    existed_before: set[str] = set()
    for relative in sorted(paths_to_backup):
        source = root / relative
        if source.exists() and source.is_file():
            existed_before.add(relative)
            target = backup / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

    # Transaction phase 1: calculate every patched file in memory. Nothing in
    # the Rio checkout is changed until all semantic source checks have passed.
    planned_patches: dict[str, str] = {}
    try:
        for relative, patcher in PATCHERS.items():
            path = root / relative
            original = path.read_text(encoding="utf-8")
            planned_patches[relative] = patcher(original)
    except Exception:
        print(f"Patch validation failed; Rio source was not modified. Backup: {backup}")
        raise

    # Transaction phase 2: commit the overlay and patched text. If filesystem
    # I/O fails halfway through, restore every touched path to its original
    # state (and remove files that did not exist before the apply).
    try:
        for source, target, _ in overlay_files:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)

        for relative, patched in planned_patches.items():
            path = root / relative
            path.write_text(patched, encoding="utf-8", newline="\n")

        for relative in OBSOLETE_FILES:
            target = root / relative
            if target.is_file() or target.is_symlink():
                target.unlink()
    except Exception:
        for relative in sorted(paths_to_backup):
            target = root / relative
            backup_file = backup / relative
            if relative in existed_before and backup_file.is_file():
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(backup_file, target)
            elif relative not in existed_before and target.exists():
                if target.is_file() or target.is_symlink():
                    target.unlink()
        print(f"Patch write failed; Rio source was rolled back. Backup: {backup}")
        raise

    print("Automexia Terminal v0.3.13 rustfmt-normalized integration applied.")
    print(f"Backup: {backup}")
    print(f"Expected upstream integration target: {UPSTREAM_TARGET}")
    print("Next: cargo fmt --all -- --check")
    print("Then: cargo check -p rioterm --all-targets")
    print("Then: cargo test -p rioterm -p rio-vt -p rio-backend -p rio-window -p sugarloaf -p teletypewriter")
    print("Then: cargo build -p rioterm --release")
    print("Finally: target/release/rio.exe --version")


if __name__ == "__main__":
    main()
