#!/usr/bin/env python3
from __future__ import annotations
import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def require(ok: bool, message: str) -> None:
    if not ok:
        raise SystemExit(f"FAIL: {message}")


def read(base: Path, rel: str) -> str:
    p = base / rel
    require(p.is_file(), f"missing {p}")
    return p.read_text(encoding="utf-8")


def verify_tree(base: Path) -> None:
    theme = read(base, "frontends/rioterm/src/automexia/theme.rs")
    shell = read(base, "frontends/rioterm/src/automexia/shell.rs")
    status = read(base, "frontends/rioterm/src/renderer/devops_status.rs")
    model = read(base, "frontends/rioterm/src/automexia/builtins/devops/model.rs")
    context = read(base, "frontends/rioterm/src/automexia/builtins/devops/context.rs")
    mod = read(base, "frontends/rioterm/src/automexia/mod.rs")

    require("pub mod theme;" in mod, "Automexia theme boundary is not compiled")
    require("pub mod shell;" in mod, "Automexia shell boundary is not compiled")
    require("AUTOMEXIA_UNIFIED_COLORS" in theme, "unified palette has no opt-out")
    for value in ("#04100D", "#EEF7F2", "#FF6F91", "#7CFFB2", "#FFD166", "#48A7FF", "#B58CFF", "#61E7FF"):
        require(value in theme, f"unified palette missing {value}")
    require("normalized_program" in shell and 'Some("powershell".to_string())' in shell, "default Windows shell is not normalized to explicit PowerShell")
    require('"-NoLogo".to_string()' in shell, "PowerShell launch does not suppress the stock banner")
    require("shell-integration\\\\automexia.ps1" in shell and '"-NoExit".to_string()' in shell, "interactive PowerShell does not directly source the Automexia integration")

    require("IconKind::Ubuntu" in status and "IconKind::Docker" in status and "IconKind::Git" in status, "native context icons missing")
    require("SEPARATOR_GAP" in status and "render_prompt_rows" in status, "segmented semantic-prompt visual language missing")
    require("prompt_history" in status and "MAX_PROMPT_CONTEXT_HISTORY" in status, "scrollback prompt visuals are not bounded/cached")
    require("active_prompt" in status and "immediate_segments" in status and "build_live_segments" in status, "first prompt is not live/enriched without command execution")
    require("IconKind::Path" not in status and "compact_path" not in status, "working directory is still rendered by DevOps instead of the shell prompt")
    require(all(name in status for name in ("first_prompt_has_immediate_wsl_version_and_user", "live_anchor_drives_prompt_without_semantic_row_scan", "previous_prompt_is_frozen_when_next_prompt_starts", "reflow_key_change_does_not_freeze_or_duplicate_live_prompt", "coalesced_next_prompt_freezes_previous_when_old_anchor_is_still_visible")), "live-prompt/freeze/reflow behavior lacks Rust regression coverage")
    require("Docker:" not in status and "Git:" not in status and "K8s:" not in status, "status UI still repeats category names instead of icon + value")
    require("pub user: Option<String>" in model, "status model cannot carry prompt identity")
    require('env::var("USERNAME")' in context and 'env::var("USER")' in context, "host user identity discovery missing")
    require("wsl_distribution_roots" not in context and r'\\wsl.localhost' not in context, "visual system still performs blocking WSL UNC discovery")

    # When this verifier is pointed at a fully applied checkout, verify that the
    # application actually consumes the boundaries rather than merely shipping
    # unused modules in the overlay.
    renderer_path = base / "frontends/rioterm/src/renderer/mod.rs"
    main_path = base / "frontends/rioterm/src/main.rs"
    context_path = base / "frontends/rioterm/src/context/mod.rs"
    if renderer_path.is_file():
        renderer = renderer_path.read_text(encoding="utf-8")
        require("crate::automexia::theme::effective_colors(config.colors)" in renderer, "renderer does not consume the unified Automexia palette")
        require("SemanticPrompt::Prompt" in renderer and "SemanticPrompt::PromptContinuation" in renderer and "PromptAnchor" in renderer and "shell_prompt_active" in renderer and "semantic_live_anchor" in renderer and "live_anchor" in renderer, "renderer does not use semantic-row-first/cursor-fallback live prompt geometry")
        require("extension_status_y_from_config" not in renderer, "renderer still references the obsolete fixed status lane")
    if main_path.is_file():
        main = main_path.read_text(encoding="utf-8")
        require('set_var("TERM_PROGRAM", "Automexia")' in main, "terminal identity is not Automexia")
        require("WSLENV" in main and "AUTOMEXIA_SHELL_INTEGRATION/u" in main, "Automexia identity is not propagated into nested WSL sessions")
    if context_path.is_file():
        app_context = context_path.read_text(encoding="utf-8")
        require("automexia::shell::normalized_program" in app_context, "Windows PTY launch does not use Automexia shell program normalization")
        require("automexia::shell::normalized_args" in app_context, "Windows PTY launch does not use Automexia shell argument normalization")


def verify_shell_files() -> None:
    ps = read(ROOT, "shell-integration/powershell/automexia.ps1")
    bash = read(ROOT, "shell-integration/bash/automexia.bash")
    zsh = read(ROOT, "shell-integration/zsh/automexia.zsh")
    require("Set-PSReadLineOption -Colors" in ps and "Command" in ps and "#B58CFF" in ps, "PowerShell editor/command colors are not unified")
    require("133;A" in ps and "133;P;k=c" in ps and "133;B" in ps and "133;D" in ps, "PowerShell two-row OSC 133 prompt metadata missing")
    require("$pathPrompt" in ps and "Get-AutomexiaPromptPath" in ps, "PowerShell path is not in the normal prompt line")
    require("automexia_prompt_active=MQ==" in ps and "automexia_prompt_active=MA==" in ps and "AddToHistoryHandler" in ps, "PowerShell prompt lifecycle metadata missing")
    require('"`n"' in ps, "PowerShell integration does not reserve a blank context row above editable input")
    require("PS1=" in bash and "PS0=" in bash and "38;2;181;140;255" in bash, "Bash prompt/input color integration missing")
    require("133;A" in bash and "133;P;k=c" in bash and "133;B" in bash and "133;C" in bash, "Bash two-row OSC 133 prompt/command metadata missing")
    require(r"\w" in bash and "PROMPT_DIRTRIM" in bash and "38;2;72;167;255" in bash, "Bash/WSL path is not compact/readable in the normal prompt line")
    require("automexia_prompt_active=MQ==" in bash and "automexia_prompt_active=MA==" in bash, "Bash prompt lifecycle metadata missing")
    require(r"\n\[\e]133;P;k=c" in bash, "Bash integration does not reserve a blank context row")
    require("PROMPT=" in zsh and "%F{magenta}" in zsh and "preexec" in zsh, "Zsh/macOS prompt/input color integration missing")
    require("133;A" in zsh and "133;P;k=c" in zsh and "133;B" in zsh and "133;C" in zsh, "Zsh two-row OSC 133 prompt/command metadata missing")
    require("%3~" in zsh and "%F{blue}" in zsh, "Zsh/macOS path is not compact/readable in the normal prompt line")
    require("automexia_prompt_active=MQ==" in zsh and "automexia_prompt_active=MA==" in zsh, "Zsh prompt lifecycle metadata missing")
    require(r"\n%{\e]133;P;k=c" in zsh, "Zsh integration does not reserve a blank context row")
    require("AUTOMEXIA_SHELL_INTEGRATION/u" in bash and "AUTOMEXIA_SHELL_INTEGRATION/u" in zsh, "WSL activation does not fall back to WSLENV identity")
    require("automexia_distro" in bash and "automexia_os_version" in bash, "Bash/WSL does not publish fast distro/version metadata")
    require("automexia_distro" in zsh and "automexia_os_version" in zsh, "Zsh/WSL does not publish fast distro/version metadata")
    require(r"\e]2;" in bash and r"\e]2;" in zsh, "shell integration does not keep raw WSL title metadata current")
    require("$'\\xCE\\xBB'" in bash, "Bash prompt glyph is not generated from ASCII-safe UTF-8 bytes")
    require(r"\xCE\xBB" in zsh, "Zsh prompt glyph is not generated from ASCII-safe UTF-8 bytes")
    require("[char]0x03BB" in ps, "PowerShell prompt glyph is not generated from U+03BB code point")
    require("λ" not in bash and "λ" not in zsh and "λ" not in ps, "shell prompt files still contain a literal glyph vulnerable to Windows pipeline encoding")
    installer = read(ROOT, "INSTALL-SHELL-INTEGRATION-WINDOWS.ps1")
    require("$PROFILE.CurrentUserCurrentHost" in installer, "PowerShell installer still guesses profile paths")
    require("wsl.exe --exec sh" in installer and "& wsl.exe sh -lc" not in installer, "WSL installer still performs repeated/login-shell startup")
    require("ToBase64String" in installer and "Encoding]::UTF8" in installer and "base64 -d | sh" in installer, "WSL installer does not transfer shell integration as UTF-8-safe base64")
    require("$payload | & wsl.exe" not in installer, "WSL installer still pipes Unicode shell source through Windows PowerShell native-command encoding")
    combined = "\n".join((ps, bash, zsh)).lower()
    for forbidden in ("alias docker=", "alias kubectl=", "function ax", "function kgp", "automexia-ax"):
        require(forbidden not in combined, f"shell integration introduces forbidden custom command behavior: {forbidden}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("project_root", nargs="?", type=Path)
    args = parser.parse_args()
    verify_tree(ROOT / "overlay")
    verify_shell_files()
    if args.project_root:
        verify_tree(args.project_root.resolve())
    print("PASS: unified Automexia palette, live/reflow-safe semantic-prompt context line, vector icons, user identity model, Windows shell normalization, and PowerShell/Bash/Zsh shell integration verified")


if __name__ == "__main__":
    main()
