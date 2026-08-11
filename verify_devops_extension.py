#!/usr/bin/env python3
from __future__ import annotations

import argparse
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REL = "frontends/rioterm/src/automexia/builtins/devops"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"FAIL: {message}")


def read(base: Path, rel: str) -> str:
    p = base / rel
    require(p.is_file(), f"missing {p}")
    return p.read_text(encoding="utf-8")


def verify(base: Path) -> None:
    mod = read(base, f"{REL}/mod.rs")
    context = read(base, f"{REL}/context.rs")
    model = read(base, f"{REL}/model.rs")
    semantics = read(base, f"{REL}/semantics.rs")
    api = read(base, "frontends/rioterm/src/automexia/api.rs")
    status = read(base, "frontends/rioterm/src/renderer/devops_status.rs")
    ui = read(base, "frontends/rioterm/src/automexia/ui.rs")

    # User-facing behavior.
    require("default_enabled: true" in mod, "DevOps is not enabled by default")
    require(
        all(icon in status for icon in ("IconKind::Ubuntu", "IconKind::Docker", "IconKind::Kubernetes", "IconKind::Git")),
        "native vector icon vocabulary is incomplete",
    )
    require(
        "pub const ICON_" not in mod,
        "obsolete string icon constants remain after native vector-icon migration",
    )
    for severity in ("Error", "Warning", "Success", "Info", "Debug"):
        require(severity in api, f"semantic severity missing {severity}")
    for sample in (
        "command not found",
        "crashloopbackoff",
        "imagepullbackoff",
        "containercreating",
        "finished `release` profile",
        "level=info",
        "level=debug",
        "[inf]",
        "[wrn]",
        "[dbg]",
        "error[",
        "npm err!",
        "fullyqualifiederrorid",
        "error response from daemon",
        "exited (",
    ):
        require(sample in semantics.lower(), f"semantic classifier missing behavior: {sample}")
    require("DevOps · no local context detected" not in status, "empty context still renders a noisy placeholder")
    require("pub struct PromptAnchor" in ui, "semantic prompt-row contribution anchor is missing")
    require("MAX_PROMPT_CONTEXT_HISTORY" in ui, "semantic prompt context history is not bounded")
    require("render_prompt_rows" in status and "prompt_history" in status, "DevOps context is not rendered/cached per semantic prompt row")
    require("empty_prompt_snapshot_is_cached" in status and "prompt_history_is_bounded" in status, "semantic prompt history regressions are not covered by Rust tests")
    require("first_prompt_has_immediate_wsl_version_and_user" in status, "initial prompt does not have synchronous OS/user regression coverage")
    require("active_prompt" in status and "build_live_segments" in status and "live_anchor" in status, "active prompt is not cursor-anchored/live-enriched")
    require("IconKind::Path" not in status and "compact_path" not in status, "working directory leaked back into the DevOps context row")
    require("status_y: f32" not in status, "HUD still uses a fixed top-strip/application-chrome position")
    require("draw_icon(" in status and "IconKind::Docker" in status and "IconKind::Ubuntu" in status and "IconKind::Git" in status, "HUD does not provide font-independent vector icons")
    require("wsl_value(" in status, "HUD does not reduce Ubuntu/WSL context to a compact value-only chip")
    require("compact_middle" in status and "MAX_GIT_CHARS" in status, "long branch/context labels are not bounded")
    for ch in mod:
        require(unicodedata.category(ch) != "Co", "DevOps icon vocabulary still contains private-use/Nerd-Font-only glyphs")

    # Context sources, including Windows-specific local CLI configuration roots.
    for source in (
        "KUBECONFIG",
        "DOCKER_CONTEXT",
        "DOCKER_CONFIG",
        "AWS_PROFILE",
        "AWS_CONFIG_FILE",
        "AZURE_CONFIG_DIR",
        "CLOUDSDK_CONFIG",
        "APPDATA",
        "TF_WORKSPACE",
        ".automexia-context.json",
    ):
        require(source in context, f"local context source missing: {source}")
    for helper in (
        "docker_from_automexia_json",
        "terraform_from_automexia_json",
        "git_from_automexia_json",
        "kubernetes_from_automexia_json",
        "cloud_from_automexia_json",
    ):
        require(f"fn {helper}" in context, f"nested project context support missing: {helper}")
    require("configured/local context" in model, "snapshot does not document configured-vs-connected semantics")
    require("WslContext" in model, "snapshot model has no nested WSL session context")
    require("parse_wsl_title" in context, "passive WSL title detection is missing")
    require("windows_wsl_session_view" in context, "Windows/WSL session view is missing")
    require("wsl_distribution_roots" not in context, "blocking WSL provider-root enumeration was reintroduced")
    require(r'\\wsl.localhost' not in context and r'\\wsl$' not in context, "blocking WSL UNC probing was reintroduced")
    require(".os_version" in context and ".distro" in context, "WSL distro/version does not use shell-published metadata")
    require('root.is_dir().then(|| "default".to_string())' in context, "Docker Desktop default-context fallback is missing")
    require("IconKind::Ubuntu" in status and "compact_middle" in status, "HUD cannot render active WSL/Git compact chips")
    require("refresh_pending" in status and "snapshot_revision" in status, "HUD does not reliably redraw when async discovery completes")
    require("shell_integration" in api, "generic session API lacks shell integration readiness")
    require(
        "pub use model::{CloudContext, DevOpsSnapshot};" in mod,
        "DevOps module public model surface is not narrowed to the externally used types",
    )
    require(
        "pub use model::{CloudContext, DevOpsSnapshot, KubernetesContext, WslContext};" not in mod,
        "unused KubernetesContext/WslContext public re-exports reintroduced compiler warnings",
    )

    # No custom CLI or command execution. /market is application UI and lives
    # outside this built-in; the extension itself is passive/read-only.
    forbidden = (
        "std::process::Command",
        "Command::new(",
        "std::net::",
        "TcpStream",
        "UdpSocket",
        "reqwest::",
    )
    combined = "\n".join((mod, context, model, semantics))
    for needle in forbidden:
        require(needle not in combined, f"DevOps built-in contains forbidden CLI/network behavior: {needle}")
    devops_dir = base / REL
    require(
        not any(path.name.lower().startswith(("cli", "command")) for path in devops_dir.iterdir() if path.is_file()),
        "DevOps built-in ships a custom CLI/command module",
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Verify Automexia DevOps extension behavior and architecture")
    parser.add_argument("project_root", nargs="?", type=Path)
    args = parser.parse_args()
    verify(ROOT / "overlay")
    if args.project_root:
        verify(args.project_root.resolve())
    print(
        "PASS: DevOps extension provides semantic-prompt-row context UI + semantic decoration, non-blocking WSL metadata plus session-aware Docker/Kubernetes/cloud discovery, "
        "bounded prompt history, reliable async refresh completion, local read-only discovery, and no custom CLI/process/network execution"
    )


if __name__ == "__main__":
    main()
