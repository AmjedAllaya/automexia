#!/usr/bin/env python3
"""Validate the disabled CP5.1-CP5.6 source implementation boundary."""

from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]
MAX_FILE_BYTES = 1_500_000

REQUIRED: dict[str, tuple[str, ...]] = {
    "automexia-devops/src/suggestions/mod.rs": (
        "pub const PROTOCOL_SCHEMA: u16 = 1;",
        "pub const FRAME_BYTES: usize = 1024 * 1024;",
        "pub const ACTIVE_ROUTES: usize = 64;",
        "pub const CACHE_BYTES: usize = 8 * 1024 * 1024;",
        "pub fn encode_submission_frame",
        "pub fn decode_replacement_frame",
        "constant_time_eq",
        "pub struct LocalSourceBroker",
        "pub fn rank_batches",
    ),
    "automexia-ui-model/src/suggestions.rs": (
        'role: "listbox"',
        'role: "option"',
        "pub fn project_surface",
        "pub struct AnnouncementGate",
        "reduced_motion",
        "SuggestionSurfaceKind::CompactHint",
    ),
    "apps/automexia-terminal/src/automexia/suggestions/mod.rs": (
        "pub struct SuggestionBroker",
        "pub fn kill(&mut self)",
        "pub fn disable(&mut self)",
        "pub fn uninstall(&mut self)",
        'fallback: "cp1-shell-native"',
    ),
    "apps/automexia-terminal/src/automexia/suggestions/service.rs": (
        'name("automexia-suggestions".into())',
        "one-latest-per-route",
        "struct FairLatestQueue",
        "lifecycle: Mutex<()>",
        "worker.join()",
        "pub fn worker_running",
    ),
    "apps/automexia-terminal/src/automexia/suggestions/platform/windows.rs": (
        "FILE_FLAG_FIRST_PIPE_INSTANCE",
        "PIPE_REJECT_REMOTE_CLIENTS",
        "GetNamedPipeClientProcessId",
        "GetNamedPipeClientSessionId",
        "pub fn read_submission",
        "pub fn write_replacement",
    ),
    "apps/automexia-terminal/src/automexia/suggestions/platform/unix.rs": (
        "PermissionsExt",
        "0o700",
        "0o600",
        "SO_PEERCRED",
        "getpeereid",
    ),
    "apps/automexia-terminal/src/automexia/suggestions/controller.rs": (
        "pub struct SuggestionUiController",
        "SuggestionInteractionKey::Enter",
        "SuggestionInteractionOutcome::ForwardToEditor",
        "revalidate_for_acceptance",
    ),
    "apps/automexia-terminal/src/renderer/suggestions.rs": (
        "Draw-only CP5 suggestion overlay",
        "SuggestionSurfaceKind::CompactHint",
        "matched",
    ),
    "automexia-devops/tests/suggestions_contract.rs": (
        "arbitrary_frames_never_bypass_checked_decode",
        "native_replacement_round_trip_revalidates_capability_and_never_executes",
    ),
    "apps/automexia-terminal/tests/suggestions_broker.rs": (
        "one_thousand_kill_enable_cycles",
        "concurrent_submit_and_disable_completes_tickets_and_joins_workers",
        "stream_endpoint_handles_fragmented_submission",
        "ui_controller_revalidates_ownership_navigation_pointer_and_acceptance",
    ),
    "automexia-ui-model/tests/suggestions_ui.rs": (
        "surface_geometry_matrix_is_exact_and_cursor_anchored",
        "accessibility_announcements_are_coalesced_without_arbitrary_sleeps",
    ),
    "fuzz/fuzz_targets/suggestion_bridge.rs": (
        "decode_request_frame",
        "decode_submission_frame",
        "decode_replacement_frame",
        "encode_request_frame",
    ),
    "automexia-devops/benches/suggestions.rs": (
        "cp5_suggestion_ranking",
        "cp5_suggestion_frame_encode_near_limit_utf8",
        "cp5_suggestion_frame_decode_near_limit_utf8",
    ),
    "shell-integration/suggestions/powershell/automexia-suggestions.ps1": (
        "PSReadLine",
        "TabExpansion2",
        "binding-collision",
        "preview-disabled",
        "Disable-AutomexiaSuggestions",
    ),
    "shell-integration/suggestions/bash/automexia-suggestions.bash": (
        "BASH_VERSINFO[0] < 5",
        "binding-collision",
        "preview-disabled",
        "automexia_suggestions_disable",
    ),
    "shell-integration/suggestions/zsh/automexia-suggestions.zsh": (
        "is-at-least 5.8",
        "binding-collision",
        "preview-disabled",
        "automexia_suggestions_disable",
    ),
    "shell-integration/suggestions/fish/automexia-suggestions.fish": (
        "unsupported-fish",
        "binding-collision",
        "preview-disabled",
        "automexia_suggestions_disable",
        "Ctrl+Space",
    ),
    "shell-integration/suggestions/README.md": (
        "packaged as inert resources",
        "activation scaffolds, not a complete shell bridge",
        "Fish also fails closed to its native UI for non-ASCII buffers",
        "Windows PowerShell 5.1 and CMD intentionally have no adapter",
        "signed host-relay native gate",
    ),
    "shell-integration/suggestions/helper-protocol.md": (
        "Only request records are implemented by the current inert scaffolds",
        "Accept, dismiss, replace, and status remain reserved protocol kinds",
    ),
}

FORBIDDEN_GLOBAL = (
    "TcpListener",
    "UdpSocket",
    "localhost:",
    "127.0.0.1",
)
FORBIDDEN_SHELL = (
    "Invoke-WebRequest",
    "Start-Process",
    "curl ",
    "wget ",
    "ssh ",
    "Enter",
)


class Cp56Error(ValueError):
    """The CP5 implementation boundary is incomplete or unsafe."""


def validate_texts(texts: dict[str, str]) -> dict[str, int]:
    if set(texts) != set(REQUIRED):
        raise Cp56Error("CP5 implementation file set changed")
    for path, fragments in REQUIRED.items():
        text = texts[path]
        missing = [fragment for fragment in fragments if fragment not in text]
        if missing:
            raise Cp56Error(f"{path} is missing {missing[0]!r}")
        if path.endswith(".rs") and any(value in text for value in FORBIDDEN_GLOBAL):
            raise Cp56Error(f"{path} gained a forbidden network transport")
        if Path(path).suffix in {".ps1", ".bash", ".zsh", ".fish"} and any(
            value in text for value in FORBIDDEN_SHELL
        ):
            raise Cp56Error(f"{path} gained forbidden shell authority")
    return {
        "source_files": len(texts),
        "shell_adapters": 4,
        "external_shell_gates": 3,
    }


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    texts: dict[str, str] = {}
    for relative in REQUIRED:
        path = root / relative
        if path.is_symlink() or not path.is_file():
            raise Cp56Error(f"required CP5 source is missing or linked: {relative}")
        if path.stat().st_size > MAX_FILE_BYTES:
            raise Cp56Error(f"required CP5 source is oversized: {relative}")
        texts[relative] = path.read_text(encoding="utf-8")
    counts = validate_texts(texts)
    for active in (
        "shell-integration/powershell/automexia.ps1",
        "shell-integration/bash/automexia.bash",
        "shell-integration/zsh/automexia.zsh",
        "shell-integration/fish/automexia.fish",
    ):
        if "suggestions/" in (root / active).read_text(encoding="utf-8"):
            raise Cp56Error(f"preview adapter was activated by default: {active}")
    return counts


def main() -> int:
    try:
        counts = validate_repository()
    except (Cp56Error, OSError, UnicodeError) as error:
        print(f"CP5 implementation validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: CP5 source remains bounded and preview-disabled "
        f"({counts['source_files']} files, {counts['shell_adapters']} adapters, "
        f"{counts['external_shell_gates']} external shell gates)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
