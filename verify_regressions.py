#!/usr/bin/env python3
from pathlib import Path
import subprocess
import sys
import tempfile

import verify_patcher as base

ROOT = Path(__file__).resolve().parent
mod = base.mod
fixtures = base.fixtures

grid_path = "frontends/rioterm/src/grid_emit.rs"
grid_before = fixtures[grid_path]
grid_after = mod.patch_grid_emit(grid_before)

short_old = """                } else if let Some(tag) = hint_tag {
                    cell_fg_hinted(tag, renderer)
                } else {
                    cell_fg(sq, style, renderer, term_colors)
                }
"""
short_new = """                } else if let Some(tag) = hint_tag {
                    cell_fg_hinted(tag, renderer)
                } else {
                    semantic_or_cell_fg(semantic_fg, sq, style, renderer, term_colors)
                }
"""

# v0.2.1 regression: a mixed branch state must be repaired rather than rejected.
mixed_grid = grid_after.replace(short_new, short_old, 1)
assert mixed_grid != grid_after, "mixed-state fixture did not change"
assert mod.patch_grid_emit(mixed_grid) == grid_after, "mixed short-circuit state was not repaired"

# v0.3.0 -> v0.3.1 regression: the semantic helper already exists but has
# only Error/Warning/Success. Upgrade it in-place rather than nesting a second
# semantic_row_fg definition.
info_debug_arms = """        crate::automexia::api::SemanticSeverity::Info =>
            Some(normalized_to_u8(renderer.named_colors.cyan)),
        crate::automexia::api::SemanticSeverity::Debug =>
            Some(normalized_to_u8(renderer.named_colors.blue)),
"""
v030_grid = grid_after.replace(info_debug_arms, "", 1)
assert v030_grid != grid_after, "v0.3.0 semantic helper fixture did not change"
v031_grid = mod.patch_grid_emit(v030_grid)
assert v031_grid == grid_after, "v0.3.0 semantic helper was not upgraded in place"
assert v031_grid.count("fn semantic_row_fg(") == 1, "semantic helper upgrade created duplicate functions"

# Session-metadata upgrade regressions. Existing v0.3.10 checkouts already
# have cwd/title/distro/version but not the v0.3.12 shell/prompt readiness
# fields used for cursor-anchored live prompt geometry. Older v0.3.7/v0.3.1
# shapes must still upgrade in one pass.
renderable_path = "frontends/rioterm/src/context/renderable.rs"
renderable_final = mod.patch_renderable(fixtures[renderable_path])
shell_fields = """    /// Whether Automexia shell integration announced itself for this session.
    pub shell_integration: bool,
    /// Whether the shell is currently waiting for editable prompt input.
    pub shell_prompt_active: bool,
"""
shell_init = """            shell_integration: false,
            shell_prompt_active: false,
"""
renderable_v0310 = renderable_final.replace(shell_fields, "", 1).replace(shell_init, "", 1)
assert renderable_v0310 != renderable_final
assert mod.patch_renderable(renderable_v0310) == renderable_final

metadata_fields = """    /// Optional shell-published distro metadata from OSC 1337 SetUserVar.
    pub shell_distro: Option<String>,
    /// Optional shell-published OS version metadata.
    pub shell_os_version: Option<String>,
"""
metadata_init = """            shell_distro: None,
            shell_os_version: None,
"""
renderable_v037 = renderable_v0310.replace(metadata_fields, "", 1).replace(metadata_init, "", 1)
assert renderable_v037 != renderable_final
assert mod.patch_renderable(renderable_v037) == renderable_final

renderable_v031 = renderable_v037.replace(
    """    /// Raw terminal/OSC title captured with the same snapshot. Unlike the
    /// configurable window title, this preserves child-session metadata
    /// such as the standard WSL `user@host:/path` title.
    pub terminal_title: String,
""",
    "",
    1,
).replace("            terminal_title: String::new(),\n", "", 1)
assert renderable_v031 != renderable_final
assert mod.patch_renderable(renderable_v031) == renderable_final

# Warning-cleanup patchers must also repair partial/mixed states so a user can
# rerun v0.3.2 over a checkout manually edited from the compiler suggestions.
win_path = "rio-window/src/platform_impl/windows/util.rs"
win_before = fixtures[win_path]
win_partial = win_before.replace("dwStyle", "dw_style", 1).replace("pointerId", "pointer_id", 1)
win_after = mod.patch_windows_util_warnings(win_before)
assert mod.patch_windows_util_warnings(win_partial) == win_after

sugar_path = "sugarloaf/src/renderer/mod.rs"
sugar_before = fixtures[sugar_path]
sugar_after = mod.patch_sugarloaf_warnings(sugar_before)
sugar_partial = sugar_before.replace(
    "        if let ImageTexture::Wgpu { view, .. } = &bg_tex.gpu {",
    "        {\n            let ImageTexture::Wgpu { view, .. } = &bg_tex.gpu;",
    1,
)
assert mod.patch_sugarloaf_warnings(sugar_partial) == sugar_after

title_path = "frontends/rioterm/src/context/title.rs"
title_before = fixtures[title_path]
title_after = mod.patch_title_warnings(title_before)
title_partial = title_before.replace(
    "    #[cfg(unix)]\n    let program =",
    "    #[cfg(not(unix))]\n    let _ = context;\n\n    #[cfg(unix)]\n    let program =",
    1,
)
assert mod.patch_title_warnings(title_partial) == title_after

# v0.3.6 -> v0.3.8 Windows shell launch migration: the previous Automexia
# normalization patched only args. Upgrade it in place so the implicit Rio
# `None => powershell` default becomes explicit and can receive -NoLogo.
context_path = "frontends/rioterm/src/context/mod.rs"
context_v037 = mod.patch_context_session(fixtures[context_path])
previous_shell_call = """            let automexia_shell_args = crate::automexia::shell::normalized_args(
                config.shell.program.as_deref(),
                &config.shell.args,
            );
            pty = match create_pty(
                config.shell.program.as_deref(),
                automexia_shell_args,
"""
current_shell_call = """            let automexia_shell_program =
                crate::automexia::shell::normalized_program(config.shell.program.as_deref());
            let automexia_shell_args = crate::automexia::shell::normalized_args(
                automexia_shell_program.as_deref(),
                &config.shell.args,
            );
            pty = match create_pty(
                automexia_shell_program.as_deref(),
                automexia_shell_args,
"""
context_v036 = context_v037.replace(current_shell_call, previous_shell_call, 1)
assert context_v036 != context_v037, "v0.3.6 shell-launch fixture did not change"
assert mod.patch_context_session(context_v036) == context_v037, "v0.3.6 shell launch was not upgraded in place"

# v0.3.6 -> v0.3.7 UI migration: the previous fixed extension-status lane
# reserved 24px above the grid. v0.3.7 renders context on blank OSC-133 prompt
# rows, so upgrades must restore Rio's stock navigation padding exactly.
utils_path = "frontends/rioterm/src/renderer/utils.rs"
utils_stock = fixtures[utils_path]
utils_v036_body = r'''#[inline]
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
'''
utils_v036 = "use crate::constants;\nuse rio_backend::config::navigation::Navigation;\n" + utils_v036_body
assert mod.patch_renderer_utils(utils_v036) == utils_stock, "v0.3.6 fixed status lane was not removed during prompt-row migration"

# v0.2.x main-module layout upgrades to the v0.3 Automexia platform boundary.
old_main = "mod context;\nmod extensions;\nmod global_hotkey;\n"
new_main = mod.patch_main(old_main)
assert "mod automexia;" in new_main and "mod extensions;" not in new_main
assert mod.patch_main(new_main) == new_main

# Comment-only upstream drift must not break the semantic regular-glyph patch.
drift_grid = grid_before.replace(
    "                    // Hint-fg wins over the cell's own fg, matching\n",
    "                    // Search hint foreground has priority here.\n",
    1,
)
drift_after = mod.patch_grid_emit(drift_grid)
assert "semantic_or_cell_fg(semantic_fg, src_sq, src_style, renderer, term_colors)" in drift_after

# Live-prompt regression: generated renderer must resolve the real OSC-133
# continuation row each frame, never revive the v0.3.10 "last historical row"
# shortcut that orphaned the HUD after resize/fullscreen.
renderer_path = "frontends/rioterm/src/renderer/mod.rs"
renderer_final = mod.patch_renderer(fixtures[renderer_path])
assert "SemanticPrompt::PromptContinuation" in renderer_final
assert "semantic_live_anchor" in renderer_final
assert "historical_anchors.last().copied()" not in renderer_final

# rustfmt wraps the readiness snapshot and several long Automexia calls. A
# failed post-format gate must be resumable: applying the patcher again should
# recognize the formatted syntax instead of duplicating fields/methods.
renderer_rustfmt_layout = renderer_final.replace(
    "            devops_enabled: crate::automexia::runtime::is_installed(crate::automexia::builtins::devops::ID),\n",
    """            devops_enabled: crate::automexia::runtime::is_installed(
                crate::automexia::builtins::devops::ID,
            ),
""",
    1,
).replace(
    "        let enabled = crate::automexia::runtime::is_installed(crate::automexia::builtins::devops::ID);\n",
    """        let enabled = crate::automexia::runtime::is_installed(
            crate::automexia::builtins::devops::ID,
        );
""",
    1,
).replace(
    '                context.renderable_content.shell_integration = terminal.user_vars.get("automexia_shell").is_some_and(|value| value == "1");\n',
    '''                context.renderable_content.shell_integration = terminal
                    .user_vars
                    .get("automexia_shell")
                    .is_some_and(|value| value == "1");
''',
    1,
).replace(
    '                context.renderable_content.shell_prompt_active = terminal.user_vars.get("automexia_prompt_active").is_some_and(|value| value == "1");\n',
    '''                context.renderable_content.shell_prompt_active = terminal
                    .user_vars
                    .get("automexia_prompt_active")
                    .is_some_and(|value| value == "1");
''',
    1,
)
assert renderer_rustfmt_layout != renderer_final, "rustfmt renderer fixture did not change"
assert mod.patch_renderer(renderer_rustfmt_layout) == renderer_rustfmt_layout

context_rustfmt_layout = context_v037.replace(
    """            let automexia_shell_program =
                crate::automexia::shell::normalized_program(config.shell.program.as_deref());
""",
    """            let automexia_shell_program = crate::automexia::shell::normalized_program(
                config.shell.program.as_deref(),
            );
""",
    1,
)
assert context_rustfmt_layout != context_v037, "rustfmt context fixture did not change"
assert mod.patch_context_session(context_rustfmt_layout) == context_rustfmt_layout

# The generated status model must contain its title parser and retry semantics;
# these are compile/lifecycle defects that marker-only architecture checks used
# to miss.
status_overlay = (ROOT / "overlay/frontends/rioterm/src/renderer/devops_status.rs").read_text(encoding="utf-8")
assert "fn parse_shell_title(" in status_overlay
assert "RefreshSubmission::Busy =>" in status_overlay
assert "self.request_in_flight = false;" in status_overlay


def write_mock_rio(root: Path, *, broken_grid: bool = False) -> dict[str, str]:
    root.mkdir(parents=True, exist_ok=True)
    (root / "Cargo.toml").write_text("[workspace]\nmembers = []\n", encoding="utf-8")
    snapshots: dict[str, str] = {}
    for relative, content in fixtures.items():
        if broken_grid and relative == grid_path:
            content = content.replace(
                "                    cell_fg(sq, style, renderer, term_colors)\n",
                "                    incompatible_fg_shape(sq, style, renderer, term_colors)\n",
            )
        target = root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8", newline="\n")
        snapshots[relative] = content
    return snapshots


# Full installer integration: all patchers + overlay, twice, with external backups.
with tempfile.TemporaryDirectory(prefix="automexia-verify-") as tmp:
    tmp_path = Path(tmp)
    rio = tmp_path / "rio"
    write_mock_rio(rio)
    cmd = [sys.executable, str(ROOT / "apply_automexia.py"), str(rio)]

    first = subprocess.run(cmd, text=True, capture_output=True)
    assert first.returncode == 0, first.stdout + first.stderr
    warning_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_warning_cleanup.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert warning_check.returncode == 0, warning_check.stdout + warning_check.stderr
    architecture_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_architecture.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert architecture_check.returncode == 0, architecture_check.stdout + architecture_check.stderr
    devops_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_devops_extension.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert devops_check.returncode == 0, devops_check.stdout + devops_check.stderr
    visual_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_visual_system.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert visual_check.returncode == 0, visual_check.stdout + visual_check.stderr
    source_quality_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_source_quality.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert source_quality_check.returncode == 0, source_quality_check.stdout + source_quality_check.stderr

    # Reproduce the bootstrap ordering that exposed the original false failure:
    # apply, rustfmt line wrapping, then architecture verification. The second
    # installer run below also proves this formatted checkout can be resumed.
    renderer_source = rio / renderer_path
    formatted_renderer = renderer_source.read_text(encoding="utf-8").replace(
        '                context.renderable_content.shell_integration = terminal.user_vars.get("automexia_shell").is_some_and(|value| value == "1");\n',
        '''                context.renderable_content.shell_integration = terminal
                    .user_vars
                    .get("automexia_shell")
                    .is_some_and(|value| value == "1");
''',
        1,
    ).replace(
        '                context.renderable_content.shell_prompt_active = terminal.user_vars.get("automexia_prompt_active").is_some_and(|value| value == "1");\n',
        '''                context.renderable_content.shell_prompt_active = terminal
                    .user_vars
                    .get("automexia_prompt_active")
                    .is_some_and(|value| value == "1");
''',
        1,
    )
    assert formatted_renderer != renderer_source.read_text(encoding="utf-8")
    renderer_source.write_text(formatted_renderer, encoding="utf-8", newline="\n")
    formatted_architecture_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_architecture.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert formatted_architecture_check.returncode == 0, (
        formatted_architecture_check.stdout + formatted_architecture_check.stderr
    )
    first_state = {
        relative: (rio / relative).read_text(encoding="utf-8") for relative in fixtures
    }
    for source in (ROOT / "overlay").rglob("*"):
        if source.is_file():
            assert (rio / source.relative_to(ROOT / "overlay")).is_file()

    second = subprocess.run(cmd, text=True, capture_output=True)
    assert second.returncode == 0, second.stdout + second.stderr
    warning_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_warning_cleanup.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert warning_check.returncode == 0, warning_check.stdout + warning_check.stderr
    architecture_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_architecture.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert architecture_check.returncode == 0, architecture_check.stdout + architecture_check.stderr
    devops_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_devops_extension.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert devops_check.returncode == 0, devops_check.stdout + devops_check.stderr
    visual_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_visual_system.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert visual_check.returncode == 0, visual_check.stdout + visual_check.stderr
    source_quality_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_source_quality.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert source_quality_check.returncode == 0, source_quality_check.stdout + source_quality_check.stderr
    second_state = {
        relative: (rio / relative).read_text(encoding="utf-8") for relative in fixtures
    }
    assert first_state == second_state, "full installer changed source on second apply"

    backups = sorted((tmp_path / "rio.backups").glob("automexia-terminal-v0.3.13-*"))
    assert len(backups) == 2, f"expected two external backups, found {len(backups)}"




# v0.2.x compiled extension namespace must be migrated away, not kept as a
# permanent compatibility shim. Simulate an already-customized checkout with
# legacy call sites and files, then prove v0.3 normalizes everything into
# crate::automexia and retires the obsolete files transactionally.
with tempfile.TemporaryDirectory(prefix="automexia-legacy-migrate-") as tmp:
    tmp_path = Path(tmp)
    rio = tmp_path / "rio"
    write_mock_rio(rio)

    # Start from already-patched shapes so semantic insertion markers are
    # present, then rewrite ownership references to the old v0.2 namespace.
    legacy_refs = {
        "frontends/rioterm/src/renderer/mod.rs": [
            ("crate::automexia::runtime::", "crate::extensions::manager::"),
            ("crate::automexia::builtins::devops::ID", "crate::extensions::devops::ID"),
        ],
        "frontends/rioterm/src/renderer/command_palette.rs": [
            ("MarketItem", "ExtensionMarketItem"),
            ("use crate::automexia::marketplace::ExtensionMarketItem;", "use crate::extensions::manager::ExtensionMarketItem;"),
        ],
        "frontends/rioterm/src/router/mod.rs": [
            ("crate::automexia::runtime::toggle", "crate::extensions::manager::toggle"),
            ("crate::automexia::runtime::market_items", "crate::extensions::manager::market_items"),
        ],
        "frontends/rioterm/src/grid_emit.rs": [
            ("crate::automexia::runtime::classify_row_text", "crate::extensions::devops::classify_row_text"),
            ("crate::automexia::api::SemanticSeverity", "crate::extensions::devops::SemanticSeverity"),
        ],
    }
    for relative, replacements in legacy_refs.items():
        current = mod.PATCHERS[relative](fixtures[relative])
        for new, old in replacements:
            current = current.replace(new, old)
        (rio / relative).write_text(current, encoding="utf-8", newline="\n")

    legacy_main = fixtures["frontends/rioterm/src/main.rs"].replace(
        "mod context;\nmod global_hotkey;",
        "mod context;\nmod extensions;\nmod global_hotkey;",
        1,
    )
    (rio / "frontends/rioterm/src/main.rs").write_text(
        legacy_main, encoding="utf-8", newline="\n"
    )
    for relative in mod.OBSOLETE_FILES:
        target = rio / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("// legacy v0.2 compatibility source\n", encoding="utf-8")

    migrated = subprocess.run(
        [sys.executable, str(ROOT / "apply_automexia.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert migrated.returncode == 0, migrated.stdout + migrated.stderr
    architecture_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_architecture.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert architecture_check.returncode == 0, architecture_check.stdout + architecture_check.stderr
    devops_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_devops_extension.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert devops_check.returncode == 0, devops_check.stdout + devops_check.stderr
    visual_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_visual_system.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert visual_check.returncode == 0, visual_check.stdout + visual_check.stderr
    source_quality_check = subprocess.run(
        [sys.executable, str(ROOT / "verify_source_quality.py"), str(rio)],
        text=True,
        capture_output=True,
    )
    assert source_quality_check.returncode == 0, source_quality_check.stdout + source_quality_check.stderr
    for relative in mod.OBSOLETE_FILES:
        assert not (rio / relative).exists(), f"obsolete v0.2 file survived migration: {relative}"
    main_rs = (rio / "frontends/rioterm/src/main.rs").read_text(encoding="utf-8")
    assert "mod automexia;" in main_rs and "mod extensions;" not in main_rs

# Failed semantic validation must not modify source or copy overlay files.
with tempfile.TemporaryDirectory(prefix="automexia-failure-verify-") as tmp:
    tmp_path = Path(tmp)
    rio = tmp_path / "rio"
    original = write_mock_rio(rio, broken_grid=True)
    cmd = [sys.executable, str(ROOT / "apply_automexia.py"), str(rio)]

    failed = subprocess.run(cmd, text=True, capture_output=True)
    assert failed.returncode != 0, "broken source unexpectedly patched successfully"
    for relative, expected in original.items():
        actual = (rio / relative).read_text(encoding="utf-8")
        assert actual == expected, f"validation failure modified {relative}"
    for source in (ROOT / "overlay").rglob("*"):
        if source.is_file():
            assert not (rio / source.relative_to(ROOT / "overlay")).exists(), (
                "validation failure copied overlay file " + str(source)
            )

    backups = sorted((tmp_path / "rio.backups").glob("automexia-terminal-v0.3.13-*"))
    assert len(backups) == 1, "failed validation should still leave one recovery backup"

print(
    "PASS: mixed-source repair, warning-clean partial-state repair, comment drift, "
    "full installer repeatability, v0.2/v0.3.0 namespace + semantic-helper migration/removal, v0.3.6 shell-launch/status-lane migration, semantic-prompt live/reflow call sites, applied source-quality gates, external backups, and transactional validation failure behavior verified"
)
