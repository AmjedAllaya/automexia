#!/usr/bin/env python3
"""Validate the disabled CP5.1-CP5.6 source implementation boundary."""

from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]
MAX_FILE_BYTES = 1_500_000

REQUIRED: dict[str, tuple[str, ...]] = {
    "automexia-command-productivity/src/suggestions/mod.rs": (
        "pub const PROTOCOL_SCHEMA: u16 = 1;",
        "pub const FRAME_BYTES: usize = 1024 * 1024;",
        "pub const ACTIVE_ROUTES: usize = 64;",
        "pub const CACHE_BYTES: usize = 8 * 1024 * 1024;",
        "pub fn encode_submission_frame",
        "pub fn decode_replacement_frame",
        "EDITOR_RESPONSE_TIMEOUT_MS",
        "constant_time_eq",
        "pub struct LocalSourceBroker",
        "pub fn rank_batches",
    ),
    "automexia-command-productivity/src/suggestions/reply.rs": (
        "pub enum NativeEditorReply",
        "pub enum NativeEditorStatusCode",
        "pub fn encode_reply_frame",
        "pub fn decode_reply_frame",
        "constant_time_eq",
        "revalidate",
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
        "serve_one_suggestion_submission",
        'fallback: "cp1-shell-native"',
    ),
    "automexia-command-productivity/src/suggestions/helper.rs": (
        "pub struct HelperRequest",
        "pub struct HelperReplace",
        "pub fn encode_record",
        "pub fn decode_record",
        "adapter_generation",
    ),
    "automexia-command-productivity/src/suggestions/helper_shell.rs": (
        "pub fn encode_shell_response",
        "pub fn decode_shell_response",
        "b'A'..=b'F'",
        "MAX_RESPONSE_BYTES",
    ),
    "apps/automexia-terminal/Cargo.toml": (
        'name = "automexia-suggestion-helper"',
        'path = "src/bin/automexia-suggestion-helper.rs"',
        '{ path = "automexia-suggestion-helper" }',
    ),
    "apps/automexia-terminal/src/bin/automexia-suggestion-helper.rs": (
        "std::env::args_os().len() != 1",
        "read_helper_bootstrap",
        "connect_helper_endpoint",
        "run_helper_records",
    ),
    "apps/automexia-terminal/src/automexia/suggestions/helper_bootstrap.rs": (
        "pub struct HelperBootstrap",
        "pub enum HelperEndpointLocator",
        "BOOTSTRAP_BYTES",
        "deny_unknown_fields",
        '.field("binding", &self.binding)',
    ),
    "apps/automexia-terminal/src/automexia/suggestions/helper_endpoint.rs": (
        "pub struct FramedHelperEndpoint",
        "encode_submission_frame",
        "decode_reply_frame",
        "NativeEditorReply::Status(status)",
        "declared > SuggestionLimits::FRAME_BYTES",
    ),
    "apps/automexia-terminal/src/automexia/suggestions/endpoint_service.rs": (
        "pub fn serve_one_suggestion_submission",
        "submit_and_wait_for_ui",
        "read_submission",
        "write_reply",
        ".flush()",
    ),
    "apps/automexia-terminal/src/automexia/suggestions/helper_runner.rs": (
        "pub fn run_helper_records",
        "pub trait HelperEndpointExchange",
        "HelperStatusCode::Stale",
        "HelperStatusCode::Unavailable",
    ),
    "apps/automexia-terminal/src/automexia/suggestions/helper.rs": (
        "pub struct HelperSessionBinding",
        "pub struct HelperSessionBridge",
        "pub fn translate_request",
        "pub fn translate_replacement",
        "AcceptanceContext::from_request",
    ),
    "apps/automexia-terminal/src/automexia/suggestions/helper_transport.rs": (
        "pub fn read_helper_record",
        "pub fn write_helper_record",
        "declared > SuggestionLimits::BATCH_BYTES",
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
        "pub fn write_reply",
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
        "NativeEditorReplacement::from_candidate",
        "SuggestionInteractionOutcome::Replace",
    ),
    "apps/automexia-terminal/src/renderer/suggestions.rs": (
        "Draw-only CP5 suggestion overlay",
        "SuggestionSurfaceKind::CompactHint",
        "matched",
    ),
    "automexia-command-productivity/tests/suggestions_contract.rs": (
        "arbitrary_frames_never_bypass_checked_decode",
        "native_replacement_round_trip_revalidates_capability_and_never_executes",
    ),
    "automexia-command-productivity/tests/suggestion_app_reply.rs": (
        "authenticated_status_round_trips_and_revalidates_exact_editor_state",
        "unknown_or_unbound_status_payloads_fail_closed",
    ),
    "automexia-command-productivity/tests/suggestion_helper_protocol.rs": (
        "every_helper_record_round_trips_without_exposing_payloads_in_debug",
        "malformed_or_authority_broadening_records_fail_closed",
    ),
    "automexia-command-productivity/tests/suggestion_helper_shell_response.rs": (
        "replacement_and_status_round_trip_as_nul_free_ascii_lines",
        "malformed_lowercase_odd_non_utf8_and_authority_kinds_fail_closed",
    ),
    "apps/automexia-terminal/tests/suggestion_helper_bootstrap.rs": (
        "inherited_bootstrap_round_trips_without_debugging_locator_or_capability",
        "unknown_fields_invalid_locator_trailing_and_oversized_prefix_fail_closed",
    ),
    "apps/automexia-terminal/tests/suggestion_helper_endpoint.rs": (
        "framed_endpoint_writes_authenticated_submission_and_reads_fragmented_replacement",
        "oversized_response_prefix_is_rejected_before_payload_read",
    ),
    "apps/automexia-terminal/tests/suggestion_helper_runner.rs": (
        "persistent_runner_exchanges_multiple_requests_and_returns_only_shell_responses",
        "malformed_partial_record_and_endpoint_failure_are_fail_closed_and_redacted",
    ),
    "apps/automexia-terminal/tests/suggestion_helper_session.rs": (
        "duplicate_or_older_editor_generations_fail_before_publication",
        "replacement_requires_current_route_capability_and_generation_and_never_executes",
    ),
    "apps/automexia-terminal/tests/suggestion_helper_transport.rs": (
        "transport_accepts_every_fragmentation_and_short_write_pattern",
        "declared_limit_is_rejected_before_payload_read_or_allocation",
    ),
    "apps/automexia-terminal/tests/suggestion_publication.rs": (
        "endpoint_worker_and_ui_exchange_one_authenticated_nonexecuting_reply",
        "spoofed_dismissed_and_killed_publications_fail_closed",
        "app_route_exchange_returns_bound_status_and_rejects_oversized_input",
        "newer_publication_supersedes_waiter_and_route_limit_is_exact",
        "app_route_exchange_reads_submission_and_writes_exact_authenticated_reply",
        "wait_publication",
    ),
    "tools/ci/test_cp5_native_powershell_bridge.ps1": (
        "AnonymousPipeServerStream",
        "DisposeLocalCopyOfClientHandle",
        "ready-unbound",
        "malformed or oversized helper reply was accepted",
        "native PowerShell preview/handle/status/cleanup bridge paths",
        "ConvertFrom-AutomexiaSuggestionResponseLine",
        "636166C3A92DE282AC2DF09F9A80",
        "C0AF",
        "F4908080",
        "C280",
        "E280AE",
    ),
    "tools/ci/test_cp5_native_shell_bridge.py": (
        "decode_request",
        "native Bash helper request/replacement/stale/status paths",
        "café-€-🚀",
        "C0AF",
        "F4908080",
        "C280",
        "E280AE",
        "pass_fds",
        "automexia_suggestions_enable",
        "ready-unbound",
    ),
    "tools/ci/test_cp5_native_shell_adapters.py": (
        "native Zsh and Fish request/replacement/status editor paths",
        "café-€-🚀",
        "C0AF",
        "F4908080",
        "C280",
        "E280AE",
        "Fish response descriptor mismatch",
        "pending-reply-bytes",
        "IO_TIMEOUT_SECONDS = 10.0",
        "automexia_suggestions_enable",
        "ready-unbound",
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
        "decode_reply_frame",
        "encode_request_frame",
    ),
    "automexia-command-productivity/benches/suggestions.rs": (
        "cp5_suggestion_ranking",
        "cp5_suggestion_frame_encode_near_limit_utf8",
        "cp5_suggestion_frame_decode_near_limit_utf8",
        "cp5_suggestion_reply_encode",
        "cp5_suggestion_reply_decode",
    ),
    "shell-integration/suggestions/powershell/automexia-suggestions.ps1": (
        "PSReadLine",
        "TabExpansion2",
        "binding-collision",
        "preview-disabled",
        "Disable-AutomexiaSuggestions",
        "AUTOMEXIA_SUGGESTION_RESPONSE_HANDLE",
        "GetBufferState",
        "::Replace(",
        "SetCursorPosition",
        "Read-AutomexiaBoundedResponseLine",
        "ConvertFrom-AutomexiaSuggestionResponseLine",
        r"\p{Cc}",
        "^[1-6]$",
    ),
    "shell-integration/suggestions/bash/automexia-suggestions.bash": (
        "BASH_VERSINFO[0] < 5",
        "binding-collision",
        "preview-disabled",
        "automexia_suggestions_disable",
        "AUTOMEXIA_SUGGESTION_RESPONSE_FD",
        "stale-editor-state",
        "READLINE_LINE=$prefix$insertion$suffix",
        "read -r -t 30 -n 2176",
        "b0 == 194 && b1 <= 159",
        "b0 == 216 && b1 == 156",
        "b0 == 226 && b1 == 128",
        "^[1-6]$",
    ),
    "shell-integration/suggestions/zsh/automexia-suggestions.zsh": (
        "is-at-least 5.8",
        "binding-collision",
        "preview-disabled",
        "automexia_suggestions_disable",
        "AUTOMEXIA_SUGGESTION_RESPONSE_FD",
        "stale-editor-state",
        "BUFFER=$prefix$__automexia_suggestion_decoded$suffix",
        "sysread -i $fd -s 2176 -t 30",
        "b0 == 194 && b1 <= 159",
        "b0 == 216 && b1 == 156",
        "b0 == 226 && b1 == 128",
        "$fields[3] == [1-6]",
    ),
    "shell-integration/suggestions/fish/automexia-suggestions.fish": (
        "unsupported-fish",
        "binding-collision",
        "preview-disabled",
        "automexia_suggestions_disable",
        "Ctrl+Space",
        "AUTOMEXIA_SUGGESTION_RESPONSE_FD",
        "complete -C",
        "commandline --replace",
        "stale-editor-state",
        "</dev/fd/4",
        "--nchars 2176",
        "set -l fish_read_limit 2176",
        "set -l fish_read_limit 4096",
        "complete -C \"$line\" | while read --local --line completion",
        "$b0 -ne 194 -o $b1 -gt 159",
        "$b0 -ne 216 -o $b1 -ne 156",
        "$b0 -eq 226 -a $b1 -eq 128",
    ),
    "shell-integration/suggestions/README.md": (
        "packaged as inert resources",
        "not an activated product bridge",
        "not silently disabled.",
        "current-buffer revalidation, and one-shot native replacement",
        "Windows PowerShell 5.1 and CMD intentionally have no adapter",
        "signed host-relay native gate",
    ),
    "shell-integration/suggestions/helper-protocol.md": (
        "nonzero adapter generation (`u64`)",
        "AXSR1<TAB>R<TAB>generation",
        "generation and exact byte span with the request",
        "never sends Enter",
    ),
}

FORBIDDEN_GLOBAL = (
    "TcpListener",
    "UdpSocket",
    "localhost:",
    "127.0.0.1",
)
FORBIDDEN_HELPER = (
    "std::env::var",
    "std::env::vars",
    "println!",
    "eprintln!",
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
        if path.endswith("automexia-suggestions.fish") and (
            "commandline --function repaint" in text
            or 'AUTOMEXIA_SUGGESTION_REQUEST_FD" != 3' not in text
            or 'AUTOMEXIA_SUGGESTION_RESPONSE_FD" != 4' not in text
        ):
            raise Cp56Error("Fish bridge lost fixed handles or reintroduced blocking repaint")
        if path.endswith("automexia-suggestion-helper.rs") and any(
            value in text for value in FORBIDDEN_HELPER
        ):
            raise Cp56Error(f"{path} gained argv/environment/output authority")
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
