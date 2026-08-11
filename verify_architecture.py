#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parent

REQUIRED_OVERLAY = [
    "frontends/rioterm/src/automexia/mod.rs",
    "frontends/rioterm/src/automexia/api.rs",
    "frontends/rioterm/src/automexia/state.rs",
    "frontends/rioterm/src/automexia/runtime.rs",
    "frontends/rioterm/src/automexia/ui.rs",
    "frontends/rioterm/src/automexia/theme.rs",
    "frontends/rioterm/src/automexia/shell.rs",
    "frontends/rioterm/src/automexia/marketplace.rs",
    "frontends/rioterm/src/automexia/builtins/mod.rs",
    "frontends/rioterm/src/automexia/builtins/devops/mod.rs",
    "frontends/rioterm/src/automexia/builtins/devops/model.rs",
    "frontends/rioterm/src/automexia/builtins/devops/context.rs",
    "frontends/rioterm/src/automexia/builtins/devops/semantics.rs",
    "frontends/rioterm/src/renderer/devops_status.rs",
]

OBSOLETE_EXTENSION_FILES = [
    "frontends/rioterm/src/extensions/mod.rs",
    "frontends/rioterm/src/extensions/manager.rs",
    "frontends/rioterm/src/extensions/devops.rs",
    "frontends/rioterm/src/automexia/builtins/devops.rs",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"FAIL: {message}")


def read(base: Path, relative: str) -> str:
    path = base / relative
    require(path.is_file(), f"missing {path}")
    return path.read_text(encoding="utf-8")


def verify_tree(base: Path) -> None:
    runtime = read(base, "frontends/rioterm/src/automexia/runtime.rs")
    renderer = read(base, "frontends/rioterm/src/renderer/devops_status.rs")
    devops_mod = read(base, "frontends/rioterm/src/automexia/builtins/devops/mod.rs")
    context = read(base, "frontends/rioterm/src/automexia/builtins/devops/context.rs")
    semantics = read(base, "frontends/rioterm/src/automexia/builtins/devops/semantics.rs")
    state = read(base, "frontends/rioterm/src/automexia/state.rs")
    api = read(base, "frontends/rioterm/src/automexia/api.rs")
    marketplace = read(base, "frontends/rioterm/src/automexia/marketplace.rs")
    ui = read(base, "frontends/rioterm/src/automexia/ui.rs")
    theme = read(base, "frontends/rioterm/src/automexia/theme.rs")
    shell = read(base, "frontends/rioterm/src/automexia/shell.rs")

    # IO/thread ownership: rendering only submits bounded refresh requests and
    # reads cached snapshots; context discovery belongs to the Automexia worker.
    require("automexia-extension-worker" in runtime, "extension discovery worker is missing")
    require("ensure_background_services" in runtime, "extension worker is not prewarmed outside HUD refresh")
    require("mpsc::sync_channel" in runtime, "bounded extension worker queue is missing")
    require("try_send" in runtime, "non-blocking worker submission is missing")
    require("DEVOPS_GENERATION" in runtime, "cached snapshot generation is missing")
    require("DEVOPS_CONTEXT_CACHE_LIMIT" in runtime, "bounded per-context cache is missing")
    require("VecDeque<(usize, u32, SessionFacts, DevOpsSnapshot)>" in runtime, "DevOps cache is not scoped per terminal session with completion revisions")
    require("DEVOPS_COMPLETION_COUNTER" in runtime, "session-local discovery completion revisions are missing")
    require("RefreshSubmission::Busy" in renderer, "renderer lacks fair retry when the worker queue is busy")
    require("request_devops_refresh" in renderer, "renderer does not request asynchronous refresh")
    require("devops_generation" in renderer, "renderer does not use generation-based cache sync")
    require("snapshot_revision" in renderer, "renderer cannot distinguish another pane's discovery completion")
    require("cached_session.as_ref() != Some(session)" in renderer, "renderer can accept a stale completion after the active shell/session changes")
    require("SessionFacts" in renderer, "renderer does not pass generic session facts to extensions")
    require("refresh_pending" in renderer, "renderer has no bounded redraw polling for async discovery completion")
    require("devops::detect" not in renderer, "renderer still performs synchronous DevOps discovery")
    require("std::fs" not in renderer and "fs::" not in renderer, "renderer directly performs filesystem IO")
    require("pub struct PromptAnchor" in ui, "generic Automexia semantic-prompt UI anchor is missing")
    require("MAX_PROMPT_CONTEXT_HISTORY" in ui, "prompt-context history is not bounded")
    require("PromptAnchor" in renderer and "render_prompt_rows" in renderer and "live_anchor" in renderer, "DevOps UI lacks cursor-anchored live prompt geometry")
    require("prompt_history" in renderer, "DevOps prompt context does not travel through bounded scrollback UI history")
    require("status_y: f32" not in renderer, "DevOps HUD still owns a fixed application-chrome coordinate")
    require("AUTOMEXIA_UNIFIED_COLORS" in theme, "application-owned unified palette is missing")
    require("normalized_args" in shell, "application-owned shell launch normalization is missing")
    require("let y = 8.0" not in renderer and "let y = 8;" not in renderer, "DevOps HUD still paints in Rio's title/navigation strip")
    utils_path = base / "frontends/rioterm/src/renderer/utils.rs"
    if utils_path.is_file():
        utils = utils_path.read_text(encoding="utf-8")
        require("extension_status_y_from_config" not in utils, "obsolete fixed extension-status lane survived semantic-prompt migration")
        require("EXTENSION_STATUS_BAR_HEIGHT" not in utils, "terminal grid still reserves obsolete fixed extension chrome")

    # Built-in separation: manifest, model, local discovery, and pure semantics
    # are independent files. This keeps the extension contribution contract
    # small and prepares it for a future sandbox host.
    require("mod context;" in devops_mod, "DevOps context service is not separated")
    require("mod model;" in devops_mod, "DevOps model is not separated")
    require("mod semantics;" in devops_mod, "DevOps semantic classifier is not separated")
    require("pub use context::detect" in devops_mod, "DevOps discovery is not exported through its boundary")
    require("pub use semantics::classify_row_text" in devops_mod, "semantic classifier is not exported through boundary")
    require("std::process::Command" not in context and "Command::new" not in context, "DevOps discovery spawns local commands")
    require("std::net" not in context and "reqwest" not in context, "DevOps discovery performs network access")
    require("classify_row_text" in semantics, "semantic classifier is missing")
    require("pub struct SessionFacts" in api, "generic terminal session facts are missing from the extension API")
    require("pub shell_integration: bool" in api, "session API lacks shell-integration readiness")
    require("terminal_title" in api.lower() or "raw terminal/osc title" in api.lower(), "session API does not document raw terminal title semantics")
    require("parse_wsl_title" in context, "passive nested WSL session detection is missing")
    require("windows_wsl_session_view" in context, "Windows-to-WSL session bridge is missing")
    require(r'\\wsl.localhost' not in context and r'\\wsl$' not in context, "blocking WSL UNC provider probing remains in extension discovery")
    require(".os_version" in context and ".distro" in context, "fast shell-published WSL metadata bridge is missing")

    for capability in (
        "FilesystemRead",
        "EnvironmentRead",
        "TerminalOutputRead",
        "UiOverlay",
        "Clipboard",
        "ProcessSpawn",
        "Network",
    ):
        require(capability in api, f"capability vocabulary missing {capability}")

    manifest_start = devops_mod.find("pub const MANIFEST")
    require(manifest_start >= 0, "DevOps capability manifest is missing")
    manifest_end = devops_mod.find("};", manifest_start)
    require(manifest_end > manifest_start, "DevOps capability manifest is malformed")
    manifest = devops_mod[manifest_start:manifest_end]
    for capability in ("FilesystemRead", "EnvironmentRead", "TerminalOutputRead", "UiOverlay"):
        require(f"Capability::{capability}" in manifest, f"DevOps capability is undeclared: {capability}")
    require("Capability::ProcessSpawn" not in manifest, "DevOps must not request process-spawn capability")
    require("Capability::Network" not in manifest, "DevOps must not request network capability")
    require("default_enabled: true" in manifest, "first-party DevOps extension is not enabled by default")

    normalized_state = "".join(state.split())
    require('join("automexia").join("extensions")' in normalized_state, "Automexia extension state is not namespaced")
    require("legacy_root_dir" in state, "v0.2 activation migration path is missing")
    require('"disabled"' in state, "default-enabled extensions have no persistent explicit-disable state")
    require("MarketItem" in marketplace, "marketplace model is not owned by Automexia")

    for relative in OBSOLETE_EXTENSION_FILES:
        require(not (base / relative).exists(), f"obsolete compiled extension source survived: {relative}")


def verify_applied_call_sites(project: Path) -> None:
    main_rs = read(project, "frontends/rioterm/src/main.rs")
    require("mod automexia;" in main_rs, "applied checkout does not enable Automexia platform module")
    require("mod extensions;" not in main_rs, "applied checkout still compiles obsolete extensions module")

    renderable = read(project, "frontends/rioterm/src/context/renderable.rs")
    require("pub terminal_title: String" in renderable, "raw terminal title is not cached in render snapshot")
    require("pub shell_distro: Option<String>" in renderable, "shell distro metadata is not cached in render snapshot")
    require("pub shell_os_version: Option<String>" in renderable, "shell OS-version metadata is not cached in render snapshot")
    require("pub shell_integration: bool" in renderable and "pub shell_prompt_active: bool" in renderable, "shell/prompt readiness is not cached in render snapshot")
    context_mod = read(project, "frontends/rioterm/src/context/mod.rs")
    require("pub shell_pid: u32" in context_mod, "generic shell PID/session field is missing")
    require('#[cfg(target_os = "windows")]\n        let shell_pid = 0u32;' in context_mod, "Windows session PID sentinel is missing")
    renderer_mod = read(project, "frontends/rioterm/src/renderer/mod.rs")
    normalized_renderer = "".join(renderer_mod.split())
    require("terminal.title.to_string()" in renderer_mod, "raw OSC terminal title is not snapshotted")
    require('terminal.user_vars.get("automexia_distro")' in renderer_mod, "shell distro OSC metadata is not snapshotted")
    require('terminal.user_vars.get("automexia_os_version")' in renderer_mod, "shell OS version OSC metadata is not snapshotted")
    require(
        'terminal.user_vars.get("automexia_shell")' in normalized_renderer
        and 'terminal.user_vars.get("automexia_prompt_active")' in normalized_renderer,
        "shell/prompt readiness OSC metadata is not snapshotted",
    )
    require("schedule_render_on_route(100)" in renderer_mod, "pending extension discovery does not request a bounded redraw")
    require("SemanticPrompt::Prompt" in renderer_mod, "applied renderer does not locate OSC-133 prompt rows")
    require("PromptAnchor" in renderer_mod and "SemanticPrompt::PromptContinuation" in renderer_mod and "semantic_live_anchor" in renderer_mod and "live_anchor" in renderer_mod and "rc.shell_prompt_active" in renderer_mod, "applied renderer does not use semantic-row-first/cursor-fallback live prompt geometry")
    require("render_prompt_rows" in renderer_mod, "applied renderer still uses fixed-window DevOps status rendering")

    scan = [
        "frontends/rioterm/src/main.rs",
        "frontends/rioterm/src/renderer/mod.rs",
        "frontends/rioterm/src/renderer/command_palette.rs",
        "frontends/rioterm/src/router/mod.rs",
        "frontends/rioterm/src/grid_emit.rs",
    ]
    for relative in scan:
        source = read(project, relative)
        require("crate::extensions::" not in source, f"legacy crate::extensions reference remains in {relative}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Verify Automexia architecture boundaries")
    parser.add_argument("project_root", nargs="?", type=Path)
    args = parser.parse_args()

    overlay = ROOT / "overlay"
    for relative in REQUIRED_OVERLAY:
        require((overlay / relative).is_file(), f"package overlay missing {relative}")
    require(not (overlay / "frontends/rioterm/src/extensions").exists(), "package still ships obsolete extension shim directory")
    verify_tree(overlay)

    bootstrap = (ROOT / "BOOTSTRAP-WINDOWS.ps1").read_text(encoding="utf-8")
    require('@("switch", "-c", $branch, $UpstreamTarget)' in bootstrap, "bootstrap does not branch from exact audited commit")
    require('@("switch", "-c", $branch, "origin/main")' not in bootstrap, "bootstrap still branches from moving origin/main")

    patcher = (ROOT / "apply_automexia.py").read_text(encoding="utf-8")
    for relative in OBSOLETE_EXTENSION_FILES:
        require(relative in patcher, f"patcher does not transactionally retire {relative}")

    for document in (
        "docs/ARCHITECTURE.md",
        "docs/THREADING.md",
        "docs/EXTENSION-PLATFORM.md",
        "docs/SOURCE-OWNERSHIP.md",
        "docs/QUALITY-GATES.md",
        "docs/UPSTREAM-STRATEGY.md",
        "docs/ROADMAP.md",
        "docs/DEVOPS-EXTENSION.md",
    ):
        require((ROOT / document).is_file(), f"architecture document missing: {document}")

    architecture = (ROOT / "docs/ARCHITECTURE.md").read_text(encoding="utf-8")
    for term in (
        "Terminal engine",
        "Application/runtime",
        "Extension worker",
        "Dependency rules",
        "Capability",
        "Threading model",
        "Hot-path",
        "Upstream",
    ):
        require(term in architecture, f"architecture documentation missing concept: {term}")

    if args.project_root is not None:
        project = args.project_root.resolve()
        verify_tree(project)
        verify_applied_call_sites(project)

    print(
        "PASS: Automexia engine/app/render boundaries, split DevOps services, session-scoped asynchronous discovery, "
        "non-blocking WSL metadata/path bridging, per-session completion tracking, capabilities, explicit default activation state, and exact Rio pin verified"
    )


if __name__ == "__main__":
    main()
