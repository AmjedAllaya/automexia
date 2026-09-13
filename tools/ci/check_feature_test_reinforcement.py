#!/usr/bin/env python3
"""Validate the scenario-oriented feature test reinforcement contract.

The ordinary feature assurance ledger proves ownership and linked evidence. This
second contract proves that every feature family has an explicit plan for the
test layers, independent oracles, interactions, and exit criteria that prevent
a narrow fixture from being mistaken for complete product evidence.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any

from markdown_anchors import markdown_anchors as _markdown_anchors


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CONTRACT = ROOT / "tests/assurance/feature-test-reinforcement-v1.json"

RISKS = {"critical", "high", "medium", "research"}
ASSURANCE_STATES = {
    "enforced",
    "partially-enforced",
    "controlled-external",
    "planned-only",
}
TEST_LAYERS = {
    "unit-contract",
    "boundary-table",
    "property-fuzz",
    "integration",
    "concurrency-model",
    "native-e2e",
    "visual-pixel",
    "accessibility",
    "performance-resource",
    "recovery-migration",
    "release-artifact",
    "mutation-checker",
}
ORACLES = {
    "exact-state",
    "byte-exact",
    "model-invariant",
    "side-effect-absence",
    "native-process-tree",
    "pixel-exact",
    "accessibility-tree-event",
    "resource-ceiling",
    "latency-ratchet",
    "storage-roundtrip",
    "artifact-identity",
    "human-reviewed",
}
FLAG_KEYS = {"visual", "native", "security", "persistence", "performance"}
TOP_LEVEL_KEYS = {
    "schema",
    "feature_matrix",
    "plan",
    "visual_policy",
    "allowed",
    "features",
}
FEATURE_KEYS = {
    "id",
    "risk",
    "current_assurance",
    "plan_anchor",
    "flags",
    "test_layers",
    "oracles",
    "interaction_partners",
    "needed_tests",
    "verification_reinforcements",
    "checker_reinforcements",
    "evidence_owners",
    "exit_criteria",
}
ALLOWED_KEYS = {"risks", "assurance_states", "test_layers", "oracles"}
FORBIDDEN_ABSOLUTE_CLAIMS = re.compile(
    r"(?:\b100\s*%\b|\bno issue(?:s)? (?:can|will) escape\b|\ball bugs?\b)",
    re.IGNORECASE,
)

REQUIRED_FEATURE_SCENARIO_DETAILS = {
    "command-productivity-cp22-quick-actions": {
        "needed_tests": ("Independent scalar score oracle", "contextual Unicode lowercase"),
        "verification_reinforcements": ("maximum-input scoring allocation ceiling", "allocating canary and unwind reset"),
        "checker_reinforcements": ("scalar-score and allocation evidence",),
    },
    "identity-config-migration": {
        "needed_tests": ("Unicode colour digit panic", "exact RGB/RGBA"),
        "verification_reinforcements": ("first-use zero-allocation palette", "same-host palette setup"),
        "checker_reinforcements": ("colour grammar and allocation evidence",),
    },
    "contributor-automation-quality-policy": {
        "needed_tests": ("parent-exit retained-pipe", "exact descendant identities", "joined readers"),
    },
    "prompt-context-devops-semantics": {
        "needed_tests": ("Lifecycle versus readiness", "condition polarity", "mixed failure counts", "parser-to-grid status colours", "kind-prefixed pods", "zero-count log prefixes"),
        "verification_reinforcements": ("literal status-colour oracles", "explicit ANSI", "disabled extension"),
        "checker_reinforcements": ("lifecycle/readiness semantics",),
    },
    "ecosystem-d7-cp6-proposal": {
        "needed_tests": ("pre-arming interrupts", "shared-engine ticks", "reused cancellation tokens", "worker-unwind cleanup"),
        "verification_reinforcements": ("invocation-local completion", "joined watchdogs"),
    },
    "packaging-release-provenance": {
        "needed_tests": ("recomputed-checksum SBOM privacy", "Minimal publication rejects full inventories",
                         "rehashed private paths and identifiers", "non-private retention",
                         "real ephemeral-key Minisign tamper coverage"),
        "verification_reinforcements": ("complete graph, license and file-hash preservation",),
    },
    "stabilization-release-assurance-s1-s2": {
        "needed_tests": ("content-bound dirty fingerprints", "logical artifact announcements", "pre-descent cache pruning", "unreadable source"),
        "verification_reinforcements": ("before/after source identity drift",),
    },
    "terminal-protocols-grid-history": {
        "needed_tests": (
            "Live child resize acknowledgments",
            "Real ConsoleHost editor",
            "native protocol cursor",
            "worker-owned coalesced resize transactions",
            "ConPTY history/live seams",
            "Long padded table rows",
            "No-resize probe invariance",
            "Real WSL multi-column listings",
            "exact-margin cursor cell",
            "blank soft-wrap fragments",
            "viewport identity journal",
            "parser-created selection journal",
            "hard-line journal",
            "boundary-only CMD D",
            "pre-epoch",
            "timezone or DST transitions",
            "resize followed by previous/next command navigation",
        ),
        "verification_reinforcements": (
            "native repaint content before child exit",
            "first visible cell and snapshot styles",
            "unselected control grid",
            "text, hard-break, whitespace and style faults",
            "source prompt, following-prompt boundary",
            "no shell-provided timestamp text",
            "duplicate IDs",
            "successful frame presentation",
        ),
        "checker_reinforcements": (
            "never substitute a post-exit captured stream",
            "local timezone conversion",
            "no-PTY side-effect coverage",
            "resize-navigation assurance",
            "post-present publication",
        ),
    },
    "pty-scheduler-process-lifecycle": {
        "needed_tests": (
            "Buffered native probes verify every consumed key value and ordinal",
            "Confirmed window dismissal",
            "Saturated native output",
            "fake-clock input-settle deadlines",
            "buffered-input ordering",
            "broken-pipe error-then-drop",
            "Caller-handle recovery",
            "Final-output lock contention",
            "Confirmed child-exit precedence",
            "Nonblocking pane retirement",
            "thread-local destruction gates",
            "capacity recovery",
            "parked split/local-tab exit journal",
            "Ordinary and exact Windows ConPTY",
            "broadcast-first teardown",
            "parked",
        ),
        "verification_reinforcements": (
            "500 ms desktop ceiling",
            "actual join acknowledgements",
            "no UI-thread joins",
            "surviving channels remain empty and connected",
            "restore refresh precedes visibility",
            "exact temporary-fixture process identities before close",
            "ConPTY reparenting",
            "multi-session wall-clock ceiling",
            "idempotent broadcasts",
            "no sequential deadline multiplication",
        ),
        "checker_reinforcements": (
            "dismissal-before-wait ordering",
            "ordinary Job ownership",
            "broadcast-before-join ordering",
            "exact pre-close process identity",
            "repeated-request idempotence",
        ),
    },
    "renderer-fonts-responsive-ui": {
        "needed_tests": (
            "Short inset command markers",
            "quantized contrast on all surfaces",
            "parser-created table row bands",
            "threshold-minus-one",
            "40 logical-pixel interaction targets",
            "zero PTY input",
            "WGPU and CPU",
            "fractional trackpad",
            "physical-to-logical scaling",
            "responsive reclamping",
            "full ISO local date and time",
            "compact date-time fallbacks",
            "one badge per result identity and display row",
            "measured label rectangles",
            "prompt-context paint rectangles",
            "frozen command duration",
            "post-present checkpoint publication",
            "two consecutive identical full-frame pixel digests",
            "physical surface width and height",
            "successful-present-only cache recording",
        ),
        "verification_reinforcements": (
            "42-pixel header",
            "decorative and focus roles",
            "exact row-gap pixel coverage",
            "exact marker pixel coverage",
            "full-shelf RGBA",
            "ownership before pane selection",
            "1,024-row event bound",
            "persistent idle indicator",
            "painted command datetime label",
            "terminal cells, PTY bytes",
            "every command-result draw rectangle",
            "cross-owner non-intersection",
            "zero changed channel tolerance",
            "successful present completion",
            "native dialog occlusion",
            "two identical full-frame digests",
            "full opaque on-screen client pixels",
            "skippable only after successful presentation",
        ),
        "checker_reinforcements": (
            "184-pixel default tab cap",
            "all-surface contrast requirements",
            "row-band benchmark and unchanged copy bytes",
            "marker geometry benchmarks and native inset checks",
            "modal event ownership",
            "no-fabrication behavior",
            "one-badge-per-row ownership",
            "prompt-context paint rectangles",
            "frozen command duration",
            "resize-navigation sequencing",
            "forced next-frame control handling",
            "dialog-occlusion rejection",
            "independent frame inspection",
            "physical extent identity",
            "fixed unsent editor input",
        ),
    },
    "windows-tabs-sessions-input": {
        "needed_tests": (
            "Shortcut editor double-click and F2",
            "stale queued edits",
            "separate-process restart",
            "Persistent header Back",
            "independent legacy scoring",
            "exhaustive category coverage",
            "held Enter",
            "shifted punctuation",
            "Alt+R/D clone",
            "all effective shortcut labels",
            "Complete classic palette defaults",
            "source-free shortcut chips",
            "palette-only Enter fallback",
            "Escape cancellation",
            "left-arrow Back",
            "strict-profile isolation",
            "shell-owned Ctrl+R and Ctrl+D",
            "single-message captured-target paste",
            "typed fallback and reset",
            "resizes immediately before Ctrl+Shift+Up/Down",
            "intersecting badge rectangles",
            "intersecting prompt-context rectangles",
            "broadcast-first teardown",
            "ConPTY reparenting",
        ),
        "verification_reinforcements": (
            "no clone action",
            "shared palette activation owner",
            "sibling silence and unchanged selection on rejection",
            "explicit user mappings",
            "all command-result paint rectangles",
            "visible snapshot publication",
            "six-second controlled Windows",
            "sequential deadline multiplication",
        ),
        "checker_reinforcements": (
            "resize-before-shortcut ordering",
            "no-PTY proof",
            "pre-close owned process identities",
            "application and descendant exit",
        ),
    },
}

WRAPPING_FEATURES = (
    "terminal-protocols-grid-history", "renderer-fonts-responsive-ui",
    "prompt-context-devops-semantics", "windows-tabs-sessions-input",
    "image-protocols-local-preview",
)
for _feature in WRAPPING_FEATURES:
    _details = REQUIRED_FEATURE_SCENARIO_DETAILS.setdefault(_feature, {})
    for _field, _phrase in (
        ("needed_tests", "Shared command-information wrapping"),
        ("verification_reinforcements", "actual glyph pixels, complete label bytes"),
        ("checker_reinforcements", "Reject loss of shared wrapping dispatch"),
    ):
        _details[_field] = (*_details.get(_field, ()), _phrase)

NATIVE_CONTRACT_SOURCES = {
    "repository_open": "apps/automexia-terminal/src/automexia/repository_open.rs",
    "desktop_path": "apps/automexia-terminal/src/automexia/desktop_path.rs",
    "editor": "apps/automexia-terminal/src/automexia/editor.rs",
    "directory_open": "apps/automexia-terminal/src/automexia/directory_open.rs",
    "local_tools": "apps/automexia-terminal/src/automexia/local_tools.rs",
    "local_tool_tests": "apps/automexia-terminal/src/automexia/local_tools/boundary_tests.rs",
    "local_session": "apps/automexia-terminal/src/automexia/local_tools/session.rs",
    "cli_process": "apps/automexia-terminal/src/automexia/cli_process.rs",
    "cli_completion": "apps/automexia-terminal/src/automexia/cli_process/windows_completion.rs",
    "cli_cancellation": "apps/automexia-terminal/src/automexia/cli_process/cancellation.rs",
    "guest_bridge": "shell-integration/amx-tool-bridge.py",
    "guest_native": "tools/ci/check_amx_guest_native.py",
    "google": "apps/automexia-terminal/src/automexia/google.rs",
    "browser_search": "apps/automexia-terminal/src/automexia/browser_search.rs",
    "google_native": "tools/ci/check_google_command_native.py",
    "desktop_open": "apps/automexia-terminal/src/automexia/desktop_open.rs",
    "session_cli": "apps/automexia-terminal/src/automexia/shell_integration.rs",
    "cli_main": "apps/automexia-terminal/src/main.rs",
    "hyperlinks": "apps/automexia-terminal/src/hints.rs",
    "hyperlink_scan": "apps/automexia-terminal/src/hints/scan.rs",
    "hyperlink_preview": "apps/automexia-terminal/src/hints/preview.rs",
    "hyperlink_tests": "apps/automexia-terminal/src/hints_tests.rs",
    "table_model": "automexia-ui-model/src/tables.rs",
    "table_capture": "apps/automexia-terminal/src/automexia/table_output.rs",
    "table_view": "apps/automexia-terminal/src/table_view.rs",
    "table_pixels": "apps/automexia-terminal/src/table_view_tests.rs",
    "qa": "tools/ci/qa.py",
    "qa_process": "tools/ci/qa_process.py",
    "compiler_probe": "tools/ci/rust_toolchain.py",
    "pty_worker": "rio-vt/src/performer/mod.rs",
    "pty_exit_tests": "rio-vt/src/performer/tests/resize_worker.rs",
    "shortcut_preferences": "apps/automexia-terminal/src/automexia/preferences.rs",
    "shortcut_editor": "apps/automexia-terminal/src/renderer/command_palette/shortcut_editor.rs",
    "xtask": "tools/xtask/src/main.rs",
    "palette": "apps/automexia-terminal/src/renderer/command_palette.rs",
    "screen": "apps/automexia-terminal/src/screen/mod.rs",
    "application": "apps/automexia-terminal/src/application.rs",
    "context": "apps/automexia-terminal/src/context/mod.rs",
    "router": "apps/automexia-terminal/src/router/mod.rs",
    "windows_pty": "teletypewriter/src/windows/mod.rs",
    "windows_conpty": "teletypewriter/src/windows/conpty.rs",
    "windows_pipes": "teletypewriter/src/windows/pipes.rs",
    "sugarloaf_cpu": "sugarloaf/src/renderer/cpu.rs",
    "native_driver": "tests/integration/resize-stress-windows.ps1",
    "resize_listing_tests": "rio-vt/tests/live_resize.rs",
    "resize_listing_fixture": "rio-vt/tests/fixtures/live-resize-output.sh",
    "resize_bench": "rio-vt/benches/vt_input.rs",
    "command_wrap_model": "apps/automexia-terminal/src/automexia/ui/command_info.rs",
    "command_wrap_renderer": "apps/automexia-terminal/src/renderer/command_info.rs",
    "renderer": "apps/automexia-terminal/src/renderer/mod.rs",
    "application_bench": "apps/automexia-terminal/benches/automexia_services.rs",
}


class ReinforcementError(ValueError):
    """The feature test reinforcement contract is incomplete or inconsistent."""


def _exact_keys(value: Any, expected: set[str], owner: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        raise ReinforcementError(f"{owner} must define exactly {sorted(expected)}")
    return value


def _bounded_text(value: Any, owner: str, *, minimum: int = 8, maximum: int = 600) -> str:
    if not isinstance(value, str):
        raise ReinforcementError(f"{owner} must be text")
    text = value.strip()
    if not minimum <= len(text.encode("utf-8")) <= maximum:
        raise ReinforcementError(f"{owner} must be {minimum}..{maximum} UTF-8 bytes")
    if any(character in text for character in ("\0", "\r", "\n")):
        raise ReinforcementError(f"{owner} contains a forbidden control character")
    if FORBIDDEN_ABSOLUTE_CLAIMS.search(text):
        raise ReinforcementError(f"{owner} makes an unverifiable absolute claim")
    return text


def _unique_strings(
    value: Any,
    owner: str,
    *,
    minimum: int,
    maximum: int,
    allowed: set[str] | None = None,
) -> list[str]:
    if not isinstance(value, list) or not minimum <= len(value) <= maximum:
        raise ReinforcementError(f"{owner} must contain {minimum}..{maximum} entries")
    result = [_bounded_text(item, f"{owner}[{index}]", minimum=2) for index, item in enumerate(value)]
    if len(set(result)) != len(result):
        raise ReinforcementError(f"{owner} contains duplicates")
    if allowed is not None and not set(result).issubset(allowed):
        raise ReinforcementError(f"{owner} contains unsupported values: {sorted(set(result) - allowed)}")
    return result


def _repository_path(root: Path, reference: str, owner: str) -> Path:
    path_text = reference.split("#", 1)[0].split("::", 1)[0]
    path = root / Path(path_text)
    try:
        path.resolve().relative_to(root.resolve())
    except ValueError as error:
        raise ReinforcementError(f"{owner} escapes the repository: {reference}") from error
    if not path.exists():
        raise ReinforcementError(f"{owner} references missing path: {reference}")
    return path


def _load_json(path: Path, owner: str) -> Any:
    def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ReinforcementError(f"{owner} contains duplicate key {key!r}")
            result[key] = value
        return result

    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicate_keys)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ReinforcementError(f"cannot read {owner}: {error}") from error


def _source_slice(text: str, start: str, end: str, owner: str) -> str:
    start_index = text.find(start)
    if start_index < 0:
        raise ReinforcementError(f"{owner} is missing {start!r}")
    end_index = text.find(end, start_index + len(start))
    if end_index < 0:
        raise ReinforcementError(f"{owner} is missing boundary {end!r}")
    return text[start_index:end_index]


def _require_fragments(text: str, fragments: tuple[str, ...], owner: str) -> None:
    for fragment in fragments:
        if fragment not in text:
            raise ReinforcementError(f"{owner} is missing invariant {fragment!r}")


def _require_order(text: str, fragments: tuple[str, ...], owner: str) -> None:
    cursor = 0
    for fragment in fragments:
        position = text.find(fragment, cursor)
        if position < 0:
            raise ReinforcementError(
                f"{owner} does not preserve required order at {fragment!r}"
            )
        cursor = position + len(fragment)


def _validate_shortcut_editor_sources(sources: dict[str, str]) -> None:
    # These checks guard cross-owner ordering; Rust behavioral tests remain the
    # oracle for dispatch, persistence and state, not source token presence.
    clean = {key: re.sub(r"//[^\n]*", "", value) for key, value in sources.items()}
    screen = clean["screen"]
    for start, end, downstream in [
        ("pub(crate) fn paste_from_clipboard(", "fn finish_paste(", "clipboard.get(source)"),
        ("pub fn paste(", "pub(crate) fn render_welcome(", "let search_active ="),
    ]:
        body = _source_slice(screen, start, end, "shortcut paste isolation")
        _require_order(body, ("command_palette.is_enabled()", "return", downstream), "shortcut paste isolation")
    apply = _source_slice(clean["application"], "fn apply_shortcut_edit(", "fn finish_shortcut_writes(", "shortcut transaction")
    _require_order(apply, ("registry::build(&config)", "self.user_preferences = candidate", ".update_bindings("), "shortcut transaction")
    if "update_config(" in apply or ".resize(" in apply:
        raise ReinforcementError("shortcut publication must not resize or reload fonts")
    writer = clean["shortcut_preferences"]
    flush = _source_slice(writer, "pub fn flush(", "pub fn shutdown(", "shortcut durable failure")
    _require_fragments(flush, ("!state.write_failed",), "shortcut durable failure")
    worker = _source_slice(writer, "fn writer_loop(", "#[cfg(test)]", "shortcut write receipt")
    _require_order(worker, ("state.completed = Some((revision, result))", "drop(state)", "notify();"), "shortcut write receipt")
    _require_fragments(clean["shortcut_editor"], ("revision >= expected", "editor.saving = Some(0)", "if repeat", "reset_requested"), "shortcut state")
    labels = _source_slice(clean["palette"], "pub fn set_effective_bindings(", "pub fn set_enabled(", "shortcut reload")
    _require_order(labels, ("self.shortcut_change.take()", "self.interrupt_shortcut_capture()", "self.refresh_shortcut_current()"), "shortcut reload")


def _validate_child_exit_sources(sources: dict[str, str]) -> None:
    worker = re.sub(r"//[^\n]*", "", sources["pty_worker"])
    finish = _source_slice(worker, "fn finish_child_exit(", "pub fn spawn(", "child exit publication")
    _require_order(finish, ("self.pty.next_child_event()", "return false;", "self.pty_read_bounded(",
                            "RioEvent::ChildExited", "self.terminal.lock().exit()",
                            "RioEvent::Render"), "child exit publication")
    _require_fragments(finish, ("remaining > 0 && !self.sender.shutdown_requested()",
                                "Ok(0) => break", "remaining -= processed"), "bounded final drain")
    _require_fragments(worker, ("MAX_FINAL_OUTPUT_BYTES: usize = 4 * READ_BUFFER_SIZE",
                                "buf.len().min(byte_limit - processed)",
                                "&mut buf[unprocessed..read_limit]"), "final drain byte ceiling")
    if worker.count("RioEvent::ChildExited") != 1:
        raise ReinforcementError("child exit requires one publication owner")
    spawn = worker.split("pub fn spawn(", 1)[1]
    dispatch = _source_slice(spawn, "if self.sender.shutdown_requested() {",
                             "for event in events.iter()", "child exit precedence")
    _require_order(dispatch, ("self.sender.shutdown_requested()", "self.drain_recv_channel(&mut state)",
                              "&& self.finish_child_exit(&mut state, &mut buf)",
                              "self.receiver.peek()", "self.pty_write(&mut state)"), "child exit precedence")
    if worker.count("self.finish_child_exit(&mut state, &mut buf);") != 4:
        raise ReinforcementError("every fatal I/O path must reconcile an already arrived child exit")
    _require_fragments(sources["pty_exit_tests"], (
        "confirmed_child_exit_precedes_queued_input_failure",
        "confirmed_child_exit_survives_final_read_failure",
        "child_exit_arriving_during_failed_write_is_published_once",
        "transport_failure_never_invents_a_confirmed_child_exit",
        "explicit_host_shutdown_precedes_child_exit_and_queued_input",
        "confirmed_child_exit_retains_tail_after_full_read_batch",
        "final_drain_has_exact_byte_ceiling_and_cancellation_boundary",
    ), "child exit regression owners")
    _require_fragments(re.sub(r"\s+", "", sources["pty_exit_tests"]),
                       ("assert_eq!(publications,expected,",), "exact exit sequence")


def _validate_qa_process_sources(sources: dict[str, str]) -> None:
    helper = sources['qa_process']
    _require_order(helper, ('job.assign(process)', "process.stdin.write(b'G')"), 'QA native admission')
    _require_order(helper, ('job.terminate()', 'process.wait(timeout=', 'reader.join(timeout=', 'stream.close()', '_launch_lock.release()'), 'QA native cleanup')
    _require_fragments(helper, ('CLEANUP_SECONDS = 5.0', 'CONTROL_BYTES = 32', 'CHUNK_BYTES = 8192', 'MAX_ARGUMENTS = 512', 'MAX_COMMAND_BYTES = 65536', '0x2000', 'os.killpg(process.pid, signal.SIGKILL)', '_quarantine = (process, job, readers)', 'sys.stdin.buffer.read(1)', 'state[\'code\'] = code'), 'QA bounded ownership')
    if sources['qa'].count('qa_process.run(') != 3:
        raise ReinforcementError('QA steps, version and source status must use one lifecycle owner')
    if sources['compiler_probe'].count('qa_process.run(') != 1:
        raise ReinforcementError('compiler identity must use the same bounded lifecycle owner')


def _validate_resize_listing_sources(sources: dict[str, str]) -> None:
    # Dispatch/fixture guards complement real executions; these are not an
    # independent oracle for terminal geometry or native rendering.
    listing = _source_slice(sources["resize_listing_tests"],
                            "fn native_live_wsl_eza_listing_restores_columns()",
                            "fn listing_name(", "real WSL listing regression")
    _require_fragments(listing, (
        "tempfile::Builder::new()", "std::fs::File::create_new(",
        "for height in [8, 2]", "run_fixture_sizes(",
        "ResizeDelivery::Burst, ResizeDelivery::AwaitWorkerCommit",
        "run_worker_output_sizes(", ".close()",
    ), "real WSL listing regression")
    tests = sources["resize_listing_tests"]
    _require_fragments(tests, (
        '"worker probe cannot repair output"',
        'text.matches(&format!("entry-{index:02}-")).count(),\n                1,',
    ), "listing baseline and uniqueness oracles")
    _require_fragments(sources["resize_listing_fixture"], (
        "eza --icons=always --color=always --width=100 --grid --sort=name --",
        "RESIZE-BASELINE", "steps=48",
    ), "real WSL listing producer")
    benchmark = _source_slice(sources["resize_bench"], "fn native_seam_roundtrip(",
                              "fn pane_close_repaint(", "checked seam benchmark")
    _require_fragments(benchmark, (
        "terminal.grid.history_size() < 64", "terminal.selection_to_string()",
        "expected", "terminal.snapshot_visible(",
    ), "checked seam benchmark")
    extreme = _source_slice(tests,
        "fn native_live_wsl_eza_extreme_resize_restores_columns()",
        "fn run_native_eza_listing(", "extreme native listing campaign")
    _require_fragments(extreme, (
        "(1, 1)", "(1, 2)", "(2, 1)", "(512, 96)",
        "for seed in 1..=8u64", "0..48", '"listing-wide"',
    ), "extreme native listing campaign")
    _require_fragments(listing, ("std::fs::create_dir(path)",
        '"abcdefgh".repeat(index % 5 + 1)'), "mixed long listing entries")
    extreme_bench = _source_slice(sources["resize_bench"],
        "fn extreme_resize_roundtrip(", "fn pane_close_repaint(",
        "checked extreme benchmark")
    _require_fragments(extreme_bench, (
        "terminal.grid.history_size() < 100", "terminal.selection_to_string()",
        "terminal.snapshot_visible(", "(2, 24)", "(16, 3)",
    ), "checked extreme benchmark")


def _validate_command_wrapping_sources(sources: dict[str, str]) -> None:
    # Supplemental wiring guards: runtime tests, not these strings, establish
    # complete labels, native-cell preservation and independently checked pixels.
    for owner, fragments in {
        "renderer": ("mod command_info;", "command_info::layout(", "command_info::project_images("),
        "screen": ("p.command_rows.source_row(y)", ".native_row(position.row.0.max(0) as usize)"),
        "command_wrap_model": (
            "remaining.grapheme_indices(true)", "word_boundary.or(accepted)?",
            "fn queued_wheel_events_preserve_every_wrapped_line()",
            "fn projection_never_duplicates_native_cells_for_any_visible_offset()",
            "assert!(previous.is_none_or(|previous| source > previous));",
        ),
        "command_wrap_renderer": (
            "fn narrow_prompt_band_cannot_give_all_its_width_to_the_timestamp()",
            "fn short_pane_keeps_all_context_reachable_without_native_history()",
            "fn wrapped_real_font_ink_stays_inside_narrow_panes_at_fractional_scales()",
            "projected_grid_pixels(&content, width, height)",
            "text.render_cpu_base(&mut pixels, width, height);",
            "text.render_cpu_modal(&mut pixels, width, height);",
            "assert_eq!(restored, completion.text);",
            '"metadata reaches the actual grid frame"',
            "pixels.iter().zip(original).filter(|(a, b)| a != b).count(),\n                        0,",
        ),
        "application_bench": (
            "fn command_information(c: &mut Criterion)", "    command_information\n);",
            "assert_eq!(fragment.bytes.start, ends[fragment.item]);",
            "assert_eq!(ends, values.map(str::len));",
            "assert!(fragment.x + fragment.width <= width + 0.001);",
            "assert_eq!(projection.origin(1), band.rows);",
        ),
    }.items():
        _require_fragments(sources[owner], fragments, "shared command wrapping " + owner)


def _validate_table_sources(sources: dict[str, str]) -> None:
    # Wiring guards complement parser/input/pixel tests, never replace them.
    for owner, fragments in {
        "table_model": ("MAX_TABLE_BYTES: usize = 256 * 1024", "MAX_TABLE_ROWS: usize = 256", "row.graphemes(true)", "source: Vec<String>"),
        "table_capture": (".bounds_to_display_string_bounded(", "Mode::ALT_SCREEN | Mode::MOUSE_MODE", "fn core_table_capture_uses_real_tab_stops_and_retains_cursor_history_and_copy()"),
        "table_view": ("self.close();", "WindowEvent::Ime(_)", "WindowEvent::DroppedFile(_)", "self.viewport.horizontal_thumb(", "table.visible_range(", "Effect::Consumed"),
        "table_pixels": ("fn table_view_parser_to_pixels_restores_exact_columns_after_extreme_navigation()", "assert_eq!(pixels, after.pixels(760, 260));", "line.0.y += 1.0;", "assert_eq!(row_lines, [77.0, 99.0, 121.0, 143.0]);"),
        "application": (".handle_table_window_event(&event, &mut self.router.clipboard)",),
        "screen": ("Act::ViewTableOutput => self.open_table_view()", "PaletteAction::ViewTableOutput => self.open_table_view()", "self.consume_table_key_release(key)", "self.table_view.draw("),
        "application_bench": ("    core_table_view,", "assert_eq!(table.source(), &source);", "assert_eq!(table.column_starts(), &[0, 11, 27]);"),
    }.items():
        _require_fragments(sources[owner], fragments, "focused core table " + owner)


def _validate_hyperlink_sources(sources: dict[str, str]) -> None:
    # Structural guards supplement actual parser, intent and raster assertions.
    for owner, fragments in {
        "hyperlinks": ("MAX_HINT_MATCHES: usize = 256", "MAX_HINT_BYTES: usize = 4096", "if !pressed || repeat", "KeyIntent::Activate", "pub(crate) fn safe_open_target", "self.snapshot = None;"),
        "hyperlink_scan": ("MAX_CELLS: usize = 256 * 1024", "MAX_LINE: usize = 16 * 1024", "self.dimensions !=", "grid.extras_table.get(*id) != Some(value)", "limits.set_retry_limit_in_match(10_000)", "previous_link == &link", "positions.extend(std::iter::repeat_n((position, end), c.len_utf8()))", "usize::from(cell.is_wide())", "end: positions[start + matched.len() - 1].1"),
        "hyperlink_preview": ("Copy only · blocked destination", "state.focus_in_lower_half()", "text_fit::fit_end", "fitted.display.into_owned()"),
        "hyperlink_tests": ("fn keyboard_links_real_platform_bindings_and_modal_intents()", "fn keyboard_links_one_osc_anchor_survives_wrapping_and_combining_extras()", "fn keyboard_links_cover_both_cells_of_final_wide_grapheme()", "fn keyboard_links_label_navigation_resets_destination_pan()", "assert_eq!(original, preview_pixels(&state, 760, 180))", "damaged[68 * 760] ^= 1", "fn keyboard_links_benchmark_checked_capture_navigation()", "assert_eq!(state.matches().len(), 100)"),
        "application": (".handle_hint_window_event(&event, &mut self.router.clipboard)",),
        "screen": ("self.remember_hint_key(key)", "if !self.validate_hint_snapshot()", "self.hint_route == Some(current.route_id)", "crate::hints::key_intent(", "if !crate::hints::safe_open_target(&hint_match.text)", "WindowEvent::Ime(_) | WindowEvent::Touch(_) | WindowEvent::DroppedFile(_)"),
    }.items():
        _require_fragments(sources[owner], fragments, "keyboard hyperlink " + owner)
    _require_fragments(sources["hyperlinks"], ("if label.len() > columns", "occupied.contains(&(target.start.row.0, col))"), "whole nonoverlapping hyperlink labels")
    _require_fragments(sources["hyperlink_preview"], ("state.focused_label()",), "hyperlink label preview fallback")
    _require_fragments(sources["screen"], (".label_cells()",), "shared hyperlink label placement")
    _require_fragments(sources["hyperlink_tests"], ("fn keyboard_links_right_edge_labels_are_whole_and_do_not_overlap()", "assert_eq!(state.label_cells().len(), 300)"), "bounded label tests and benchmark")
    body = _source_slice(sources["screen"], "fn process_hint_key(", "pub(crate) fn handle_hint_window_event", "hyperlink key owner")
    if body.index("self.validate_hint_snapshot()") > body.index("self.execute_hint_action("):
        raise ReinforcementError("hyperlink activation precedes stale-state validation")
    for forbidden in ('args {:?}', 'could not open {target}'):
        if forbidden in sources["screen"]:
            raise ReinforcementError("hyperlink launcher discloses untrusted target data")


def _validate_google_sources(sources: dict[str, str]) -> None:
    for owner, fragments in {
        "browser_search": ("super::google::validated_query(arguments)?", ".append_pair(provider.query_key, &query)", 'append_pair("as_sitesearch", site)', "if command.search.print_url", "fn amx_browser_search_every_route_enforces_limits_before_dispatch()", "fn amx_browser_search_benchmark_checked_routing()"),
        "google": ("MAX_QUERY_BYTES: usize = 4096", "MAX_QUERY_ARGUMENTS: usize = 256", "argument.chars().any(char::is_control)", 'url::Url::parse("https://www.google.com/search")', 'append_pair("q", &query)', "if command.print_url", "fn google_command_dispatch_preview_failure_and_debug_are_private()", "fn google_command_benchmark_checked_encoding()"),
        "google_native": ("timeout_seconds=25", "65536", '"function", "alias", "external", "disabled", "missing"', "set(Path(temporary).rglob", "actual == expected", "def test_real_search_and_docs_routes_are_offline_and_bounded(self):"),
        "desktop_open": ("target.contains('\\0')", "ShellExecuteW(", ".arg(target)", ".stdin(Stdio::null())", "with_com_apartment(|| open_windows(target))", "impl Drop for ApartmentGuard", "if result >= 0", "fn google_command_native_apartment_balances_success_failure_and_existing_mode()"),
        "session_cli": ("env::current_exe()", '"AUTOMEXIA_CLI/up"', "cmd_alias_safe"),
        "xtask": ("smoke_google_command()?;", '"tools/ci/check_google_command_native.py"', 'run_command(command, "native Google command smoke (offline preview)")'),
        "cli_main": ("CliCommand::Google(command)", "automexia::google::execute(command)", "automexia::browser_search::execute(command, false)", "automexia::browser_search::execute(command, true)"),
        "screen": ("crate::automexia::desktop_open::open(target)",),
    }.items():
        _require_fragments(sources[owner], fragments, "Google command " + owner)
    ready = _source_slice(sources["xtask"], "fn ready()", "fn dev(", "Google native readiness")
    if ready.index("build_debug_app()?") > ready.index("smoke_google_command()?"):
        raise ReinforcementError("Google native smoke must use the current built executable")


LOCAL_TOOL_CONTRACTS = {
    "repository_open": ('local_tools::run_tool("git", &args, session)', 'MAX_REMOTE_BYTES: usize = 4096', 'MAX_REMOTE_NAME_BYTES: usize = 128', '"--no-pager", "remote", "get-url", "--", remote', 'url.password().is_none()', '"github.com" | "gitlab.com"', 'components > 16', 'part.len() > 255', 'RepoPage::Issues', 'if preview', 'super::desktop_open::open', 'fn amx_repo_credentials_hosts_and_ambiguous_paths_are_never_opened()', 'fn amx_repo_benchmark_checked_remote_parsing()'),
    "directory_open": ('Kind::Directory', "if preview", "fn amx_open_preview_never_launches_and_failure_is_redacted()"),
    "desktop_path": ('local_tools::run_tool(kind.probe()', 'Self::Directory => "amx-directory"', 'Self::File => "amx-file"', 'Self::Directory => path.is_dir()', 'Self::File => path.is_file()', "if !kind.matches(&resolved)", "MAX_PATH_BYTES: usize = 4096", "all(windows_directory_component)", "stem.eq_ignore_ascii_case(name)", "fn amx_open_benchmark_checked_guest_mapping()"),
    "editor": ('Kind::File', 'read_bounded_untrusted_regular', 'MAX_CONFIG_BYTES: usize = 16 * 1024', 'preferences.version != 1', 'configured.scheme()?;', 'serde(deny_unknown_fields)', 'desktop_path::valid_text(path)', 'line > MAX_POSITION', 'column > MAX_POSITION', 'path.contains', 'ends_with(".code-workspace")', 'uri.path_segments_mut()', 'if preview', 'super::desktop_open::open', 'fn amx_edit_urls_preserve_exact_path_authority_and_coordinates()', 'fn amx_edit_preferences_are_bounded_strict_and_user_disable_wins()', 'fn amx_edit_preview_and_error_never_launch_or_disclose_diagnostics()', 'fn amx_edit_benchmark_checked_uri_encoding()'),
    "desktop_open": ('open_windows_verb(target, "explore")', 'directory_command(target, cfg!(target_os = "macos"))', 'command.arg("-R")', 'fn amx_open_directory_commands_preserve_literal_targets_and_reveal_packages()'),
    "cli_main": ("CliCommand::Open(command)", "automexia::directory_open::execute(command, session)", "CliCommand::Edit(command)", "automexia::editor::execute(command, session)"),
    "local_tools": ('"--no-config"', '"--fixed-strings"', '"--no-auto-update"', "cancellation.cancelled()", "MAX_RESULTS: usize = 1000", "MAX_FILES: usize = 32768", "relative_name(path)?", "incomplete structured search output"),
    "local_tool_tests": ("fn amx_local_structured_matches_cannot_inject_controls_or_fake_result_rows()", "fn amx_local_benchmark_checked_parsing()"),
    "local_session": ('"python3", "-I", "-c"', "guest invocation exceeds the native command-line limit"),
    "cli_process": ("CreationFlags(Default::default())", "libc::WNOWAIT", "WaitForSingleObject", "drop(lease)", "limits.stdout > 4 * 1024 * 1024", "fn amx_process_native_console_cancel_retires_the_owned_tool()"),
    "cli_cancellation": ("ACTIVE.swap(true", "SetConsoleCtrlHandler", "signal_hook::flag::register", "impl Drop for Cancellation"),
    "guest_bridge": ("object_pairs_hook=unique_fields", "start_new_session=True", "os.WNOWAIT", "selector.select(0)", "signal.signal(signal.SIGTERM, interrupted)", "os.killpg(child.pid, signal.SIGKILL)", 'len(args) != 1 or not args[0]', 'sys.executable, "-I", "-c", PATH_PROBE', 'os.path.realpath(sys.argv[1], strict=True)', "os.path.isdir(target)", "os.path.isfile(target)", 'request["program"] in {"amx-directory", "amx-file"}'),
    "guest_native": ("def test_lease_eof_termination_and_deadline_retire_exact_guest_child(self):", "os.pidfd_open", "def test_project_python_modules_cannot_execute_during_guest_launch(self):", "def test_real_ripgrep_respects_ignore_privacy_binary_and_symlink_rules(self):"),
    "google_native": ('"searches", "local", "directory", "editor"', 'expected = b"./Dockerfile"', 'def test_real_directory_preview_is_exact_and_never_writes(self):', 'plan["destination"] == target', 'if case == "directory":', 'def test_real_editor_preview_preserves_file_position_and_never_writes(self):', 'def test_real_editor_config_override_disable_and_invalid_targets(self):', 'if case == "editor":', 'AUTOMEXIA_CONFIG_HOME/pw'),
}

LOCAL_TOOL_CONTRACTS["guest_native"] += (
    "def test_explain_client_failures_and_controls_are_safe_without_downloads(self):",
    'self.assertEqual(actual, expected, "unsupported client or cache update was requested")',
    'self.assertEqual(output, b"", "failed client published partial examples")',
    'self.assertNotIn(b"fixture-client-diagnostic", errors)',
    "def test_relative_guest_path_cannot_select_project_python_before_isolation(self):",
    "def test_absolute_guest_tool_locations_keep_precedence(self):",
    '"project interpreter executed before isolated mode"',
    "def test_repository_navigation_reads_the_guest_repository_not_host_metadata(self):",
    "def test_editor_preview_resolves_guest_file_links_and_rejects_directories(self):",
    '"file probe imported untrusted project code"',
    "def test_directory_preview_resolves_guest_symlinks_without_desktop_or_writes(self):",
    'plan["destination"] == expected', '("alias/..", parent)',
    '"directory probe imported untrusted project code"',
)
LOCAL_TOOL_CONTRACTS["cli_main"] += ("CliCommand::Repo(command)", "automexia::repository_open::execute(command, session)")
LOCAL_TOOL_CONTRACTS["google_native"] += ('def test_real_repository_navigation_is_offline_exact_and_read_only(self):', 'if case == "repository":', '"missing remote must not select another remote"', '"credentials must not appear in diagnostics"')
LOCAL_TOOL_CONTRACTS["guest_bridge"] += ('args[:4] != ["--no-pager", "remote", "get-url", "--"]',)
LOCAL_TOOL_CONTRACTS["local_session"] += (
    "MAX_GUEST_HINT_BYTES: usize = 8192", "path.len() > MAX_GUEST_HINT_BYTES",
    "path.chars().any(char::is_control)", "path.split(':').enumerate()",
    "if index >= 256", "entry.starts_with('/')", "if absolute.is_empty()",
    "fn amx_local_guest_path_keeps_absolute_order_and_posix_semantics()",
    "fn amx_local_guest_path_limits_fail_closed_without_disclosing_input()",
    "fn amx_local_invalid_guest_path_is_rejected_before_executable_lookup()",
    "fn amx_local_guest_path_benchmark_checked_filtering()",
)
LOCAL_TOOL_CONTRACTS["repository_open"] += (
    "url.host_str().ok_or_else(invalid)?.to_ascii_lowercase()",
    "fn amx_repo_dns_case_is_insensitive_but_repository_case_is_preserved()",
)
LOCAL_TOOL_CONTRACTS["google_native"] += (
    "def test_real_text_search_discards_late_binary_matches(self):",
    'self.assertEqual(output, prefix + b"text.txt:2:connection refused\\n")',
    'b"x\\n" * 100000',
    "def test_requested_search_examples_have_exact_offline_destinations(self):",
    "def test_real_repository_nested_fetch_rewrite_is_read_only(self):",
    '"--push", "origin", "https://gitlab.com/different/fixture.git"',
    "def test_real_editor_uri_decodes_exactly_and_project_cannot_override_disable(self):",
    'unquote(parsed.path) == expected + ":42:7"',
    'self.assertNotEqual(code, 0, "project settings overrode user disable")',
    "ssh://git@GitHub.COM/Example-Org/Fixture-Repo.git",
    "ssh://git@GitLab.COM/Example-Group/Subgroup/Fixture-Repo.git",
)
LOCAL_TOOL_CONTRACTS["local_tools"] += (
    'let record: SearchRecord = serde_json::from_slice(row)',
    'files.len() >= MAX_FILES || files.contains_key(&data.path.text)',
    'pending.len() >= MAX_RESULTS', 'data.binary_offset.as_u64().is_some()',
    'ended[index] = Some(!data.binary_offset.is_null());',
    'ended.iter().any(Option::is_none)', 'if summarized',
    'if ended[index] == Some(false)', 'if formatted_bytes > 4 * 1024 * 1024',
)
LOCAL_TOOL_CONTRACTS["local_tool_tests"] += (
    "fn amx_local_late_binary_end_retracts_only_that_files_matches()",
    "fn amx_local_structured_lifecycle_fails_closed_before_publication()",
    "assert_eq!(result, expected_files)", "assert_eq!(result, expected_matches)",
)
LOCAL_TOOL_CONTRACTS["cli_process"] += (
    "self.completion.pin_members()", "self.completion.is_empty()?",
    "windows_completion::members_stopped(&self.members)?", "if self.reaped && tree_empty",
    "if had_lease && !self.reaped", "GetProcessHandleCount", "for _ in 0..20",
    '"native handles grew after completed capture"',
    "fn observe_identity(", "struct PinnedIdentity", '"descendant-no-pipes"',
    "for _ in 0..4", "for iteration in 0..25", "AMX_PROCESS_RELEASE",
    '"fixture must remain live until the parent pins its identity"',
    '"pinned native descendant remained live after capture; later completion={}"',
)

LOCAL_TOOL_CONTRACTS["cli_completion"] = (
    "MAX_MEMBERS: usize = 256", "QueryInformationJobObject(",
    "accounting.ActiveProcesses == 0", "JobObjectBasicProcessIdList",
    "IsProcessInJob(", "WAIT_OBJECT_0 => Ok(true)", "WAIT_TIMEOUT => Ok(false)",
    "core.get_wrap::<JobObject>().is_none()", "fn post_spawn(",
    "AssignProcessToJobObject(", "let _ = child.kill();",
    "Instant::now() < deadline", "OwnedHandle::from_raw_handle(raw)",
    "fn amx_process_completion_requires_suspended_job_owner_before_spawn()",
    "fn amx_process_completion_list_layout_matches_win32()",
)


def _validate_local_tool_sources(sources: dict[str, str]) -> None:
    for owner, fragments in LOCAL_TOOL_CONTRACTS.items():
        _require_fragments(sources[owner], fragments, "local tool " + owner)
    parser = _source_slice(sources["local_tools"], "fn render_matches(", "fn resolve_tool(", "structured search publication")
    _require_order(parser, ("ended.iter().any(Option::is_none)", "for (index, row) in pending", "if ended[index] == Some(false)", "result.push_str(&row)"), "complete nonbinary files before publication")
    bootstrap = _source_slice(sources["local_session"], 'let path = guest_tool_path', 'command.arg(request);', 'guest interpreter bootstrap')
    _require_order(bootstrap, (
        'let path = guest_tool_path(self.path.as_deref().unwrap())?;',
        'Command::new(super::resolve_tool("wsl")?)',
        'command.arg(format!("PATH={path}"));',
        '"python3", "-I", "-c"',
    ), 'validate PATH before interpreter selection')
    if 'format!("PATH={}", self.path' in sources["local_session"]:
        raise ReinforcementError("guest bootstrap restores unfiltered PATH")
    editor = _source_slice(sources["editor"], "fn selected_editor(", "pub fn execute(", "editor preference policy")
    _require_order(editor, ("configured.scheme()?;", "override_editor.unwrap_or(configured)", "selected.scheme()?;"), "disable-before-editor-override")
    _require_fragments(sources["editor"], ('uri.insert(boundary,',), "editor UNC identity")
    retire = _source_slice(sources["cli_process"], "fn retire(", "fn cleanup(", "native completion")
    _require_order(retire, ("self.completion.pin_members()", "self.inner.start_kill()", "self.members = members?"), "pin-before-termination")


def _validate_native_contract_sources(sources: dict[str, str]) -> None:
    missing = set(NATIVE_CONTRACT_SOURCES) - set(sources)
    if missing:
        raise ReinforcementError(f"native assurance sources are missing {sorted(missing)}")

    _validate_shortcut_editor_sources(sources)
    _validate_child_exit_sources(sources)
    _validate_qa_process_sources(sources)
    _validate_resize_listing_sources(sources)
    _validate_command_wrapping_sources(sources)
    _validate_table_sources(sources)
    _validate_hyperlink_sources(sources)
    _validate_google_sources(sources)
    _validate_local_tool_sources(sources)

    stress = _source_slice(sources["xtask"], "fn test_resize_stress(",
                           "if !native_gui {", "default resize stress dispatch")
    body = stress.split("{", 1)[1]
    body = re.sub(r"//[^\n]*", "", body).strip()
    if body != 'run_resize_regressions(|args| run("cargo", args))?;':
        raise ReinforcementError("default resize stress must run and propagate the native ladder")
    ladder = _source_slice(sources["xtask"], "fn run_resize_regressions(",
                           "fn test_resize_stress(", "resize regression ladder")
    for suite in ('"resize_stress"', '"resize_repaint"', '"live_resize"', '"pane_editor_resize"'):
        if ladder.count(suite) != 1:
            raise ReinforcementError("resize regression ladder lost or duplicated a required suite")
    if '"--ignored"' in ladder or '"--skip"' in ladder:
        raise ReinforcementError("resize regression ladder filters required native coverage")

    dispatch = _source_slice(
        sources["screen"], "pub fn process_key_bindings(", "match &action {",
        "shortcut dispatch work",
    )
    _require_order(
        dispatch,
        ("let logical_key =", "for i in 0..self.bindings.len()",
         "if binding.is_triggered_by", "let action = binding.action.clone();"),
        "shortcut dispatch work",
    )
    if dispatch.count("let action = binding.action.clone();") != 1:
        raise ReinforcementError("shortcut dispatch clones unmatched actions")

    palette = sources["palette"]
    labels = _source_slice(palette, "pub fn set_effective_bindings(", "pub fn set_enabled(", "effective shortcut labels")
    _require_fragments(labels, ("for command in COMMANDS", "legacy_binding_target(action)",
                               "suppresses_legacy_trigger", "SequenceResolution::NoMatch"),
                       "effective shortcut labels")
    _require_fragments(palette, (
        "icon: CommandIcon::Back,",
        "for [x1, y1, x2, y2] in BACK_ARROW_STROKES",
        "canvas.line(x1, y1, x2, y2);",
    ), "Back arrow")
    header = _source_slice(palette, "let input_icon_well =", "let text_x =", "Back header")
    _require_fragments(header, ("CommandIcon::Back,",), "Back header")
    if sources["screen"].count(".set_effective_bindings(") != 2:
        raise ReinforcementError("palette labels require construction and reload owners")

    frame = _source_slice(
        sources["screen"],
        "let force_present_for_control = self.native_test_present_after_control;",
        "// Return each panel's snapshot buffers",
        "native frame publication",
    )
    _require_fragments(
        frame,
        (
            "(!control_changed)",
            "force_present_for_control",
            "if !frame_dropped",
            "self.pending_native_snapshot = None",
        ),
        "native frame publication",
    )
    _require_order(
        frame,
        (
            "self.process_native_test_control();",
            "write_native_resize_snapshot(",
            "if should_present {",
            "self.sugarloaf.render",
            "let frame_dropped",
            "if !frame_dropped",
            "pending.publish()",
        ),
        "native frame publication",
    )

    application = sources["application"]
    close_window = _source_slice(
        application,
        "fn close_window_route",
        "fn request_application_exit",
        "window shutdown",
    )
    _require_order(
        close_window,
        ("route.window.winit_window.set_visible(false)", "manager.request_pty_shutdown()", "remove_window", "drop(route)"),
        "window shutdown",
    )
    application_exit = _source_slice(
        application,
        "fn request_application_exit",
        "fn close_window_and_maybe_exit",
        "application shutdown request",
    )
    _require_order(
        application_exit,
        ("self.router.hide_windows_for_exit()", "self.router.request_pty_shutdown()", "event_loop.exit()"),
        "application shutdown request",
    )
    final_exit = _source_slice(
        application,
        "fn exiting(&mut self",
        "#[cfg(all(",
        "final application shutdown",
    )
    _require_order(
        final_exit,
        (
            "self.router.hide_windows_for_exit()",
            "self.router.request_pty_shutdown()",
            "self.router.routes.clear()",
            "self.preference_writer.shutdown",
            "finish_shutdown(Duration::from_secs(10))",
            "std::process::exit(0)",
        ),
        "final application shutdown",
    )

    context = _source_slice(
        sources["context"],
        "pub fn request_pty_shutdown(&self) -> bool",
        "pub fn set_selection",
        "context shutdown idempotence",
    )
    _require_order(
        context,
        ("self.shutdown_requested.swap(true", "send(Msg::Shutdown)"),
        "context shutdown idempotence",
    )
    undo_restore = _source_slice(
        sources["context"], "pub fn undo_topology(", "fn undo_topology_model", "parked restore publication"
    )
    _require_order(
        undo_restore,
        ("self.undo_topology_model()", "self.current_grid_mut().update_dimensions(sugarloaf)", "self.keep_only_active_context_visible(sugarloaf)"),
        "parked restore publication",
    )
    route_quit = _source_slice(
        sources["router"], "pub fn quit(&mut self)", "pub fn open_config", "route quit"
    )
    _require_fragments(route_quit, ("context_manager.quit()",), "route quit")
    if "process::exit" in route_quit:
        raise ReinforcementError("route quit bypasses application-owned shutdown")
    if "shutdown_connection_hub()" in route_quit:
        raise ReinforcementError("route quit blocks on shared services before application exit")
    hiding = _source_slice(sources["router"], "pub fn hide_windows_for_exit", "pub fn shutdown_services", "window dismissal")
    _require_order(hiding, ("for route in self.routes.values() {", "route.window.winit_window.set_visible(false)"), "window dismissal")
    pipes = sources["windows_pipes"]
    discard = _source_slice(pipes, "pub(super) fn discard_remaining", "pub fn new", "retired native output")
    _require_order(discard, ("self.inner.wait_tag.lock()", "self.inner.discard.store(true", "self.inner.sig_buffer_not_full.notify_one()"), "retired native output")
    _require_fragments(pipes, ("&& !inner.discard.load(Ordering::SeqCst)", "if inner.discard.load(Ordering::SeqCst)", "saturated_output_drains_after_the_terminal_consumer_retires", "native_pipe_exit_preserves_final_buffered_output_before_eof"), "native drain tests and wakeup")
    pipe_read = _source_slice(pipes, "impl io::Read for EventedAnonRead", "impl Evented for EventedAnonRead", "native output EOF")
    _require_order(pipe_read, ("self.inner.wait_tag.lock()", "if self.consumer.is_empty()", "self.error_receiver.try_recv()", "self.consumer.read_to_slice(buf)"), "native output EOF")
    pty_drop = _source_slice(sources["windows_pty"], "impl Drop for Pty", "// Creates conpty", "native drop drain")
    _require_fragments(pty_drop, ("self.conout.discard_remaining()",), "native drop drain")
    pty_shutdown = _source_slice(sources["windows_pty"], "fn shutdown_owned_process_tree", "fn wait_for_job_empty", "native shutdown drain")
    _require_order(pty_shutdown, ("self.conout.discard_remaining()", "self.conin.write_all", "Duration::from_secs(2)", "terminate_managed_job()", "Duration::from_secs(3)"), "native shutdown drain")

    ordinary_pty = _source_slice(
        sources["windows_pty"],
        "pub fn create_pty(",
        "pub fn create_exact_pty(",
        "ordinary Windows PTY ownership",
    )
    if not re.search(
        r"conpty::new\(\s*None,.*?\s+true,\s*true,\s*columns,\s*rows,\s*\)",
        ordinary_pty,
        flags=re.DOTALL,
    ):
        raise ReinforcementError(
            "ordinary Windows PTY must inherit its environment and own a managed Job"
        )

    conpty_launch = sources["windows_conpty"]
    _require_fragments(
        conpty_launch,
        (
            "limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;",
            "CREATE_SUSPENDED",
        ),
        "Windows ConPTY launch",
    )
    _require_order(
        conpty_launch,
        ("CreateProcessW(", "AssignProcessToJobObject(", "ResumeThread("),
        "Windows ConPTY launch",
    )

    cpu_renderer = sources["sugarloaf_cpu"]
    cpu_frame = _source_slice(
        cpu_renderer,
        "let frame_hash = {",
        "/// Paint one primitive phase",
        "CPU frame presentation cache",
    )
    _require_fragments(
        cpu_frame,
        (
            "hash_surface_extent(&mut h, ctx.width_px, ctx.height_px);",
            "cache.can_skip_frame(frame_hash)",
            "cache.record_presented_frame(frame_hash)",
        ),
        "CPU frame presentation cache",
    )
    _require_order(
        cpu_frame,
        (
            "hash_surface_extent(&mut h, ctx.width_px, ctx.height_px);",
            "cache.can_skip_frame(frame_hash)",
            "buffer.present()",
            "cache.record_presented_frame(frame_hash)",
        ),
        "CPU frame presentation cache",
    )

    driver = sources["native_driver"]
    capture = _source_slice(
        driver,
        "public static FrameStats CaptureClientFrame(IntPtr hWnd, string outputPath)",
        "private static FrameStats CaptureClientFrameCore",
        "native stable capture",
    )
    _require_fragments(
        capture,
        (
            "previousFrame.PixelDigest",
            "current.PixelDigest",
            "two identical full-pixel captures",
        ),
        "native stable capture",
    )
    _require_fragments(
        driver,
        (
            "RequireExclusiveCaptureOwnership(hWnd);",
            'className.ToString() != "#32770"',
            "PixelDigest = pixelDigest",
            "NonOpaquePixelCount = nonOpaquePixels",
            "AMX_CAPTURE_INPUT_59217",
            "[int]$MaximumOwnedShutdownMilliseconds = 6000",
            "owned_process_tree_shutdown",
        ),
        "native visual and lifecycle driver",
    )
    shutdown = _source_slice(
        driver,
        "$script:testStage = 'application process-tree shutdown'",
        "Write-Host ($successSummary",
        "native process shutdown",
    )
    _require_order(
        shutdown,
        (
            "Get-AutomexiaOwnedProcessIds $process.Id $configRoot",
            "$process.CloseMainWindow()",
            "$shutdownTimer.ElapsedMilliseconds -lt 500",
            "if ([AutomexiaResizeDriver]::IsWindowVisible($window))",
            "$process.WaitForExit(15000)",
            "$remainingOwnedProcesses",
            "$MaximumOwnedShutdownMilliseconds",
        ),
        "native process shutdown",
    )
    _require_fragments(driver, ("$windowCloseTimer.ElapsedMilliseconds -lt 500", "if ([AutomexiaResizeDriver]::IsWindowVisible($newWindow))", "dismissal_ceiling_milliseconds = 500"), "separate native window dismissal")


def _load_native_contract_sources(root: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for owner, relative in NATIVE_CONTRACT_SOURCES.items():
        path = _repository_path(root, relative, f"native contract source {owner}")
        try:
            result[owner] = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            raise ReinforcementError(f"cannot read native contract source {owner}: {error}") from error
    return result


def _validate_exact_visual_policy(path: Path) -> None:
    policy = _load_json(path, "visual policy")
    _exact_keys(policy, {"schema", "max_channel_delta", "max_changed_pixel_ratio", "masks"}, "visual policy")
    if policy["schema"] != 1:
        raise ReinforcementError("visual policy schema must be 1")
    if policy["max_channel_delta"] != 0 or policy["max_changed_pixel_ratio"] != 0.0:
        raise ReinforcementError("deterministic visual policy must fail on one changed pixel channel")
    if policy["masks"] != []:
        raise ReinforcementError("the repository deterministic visual policy cannot mask pixels")


def validate_document(document: Any, root: Path = ROOT) -> dict[str, int]:
    contract = _exact_keys(document, TOP_LEVEL_KEYS, "feature test reinforcement contract")
    if contract["schema"] != 1:
        raise ReinforcementError("feature test reinforcement schema must be 1")

    allowed = _exact_keys(contract["allowed"], ALLOWED_KEYS, "allowed vocabulary")
    declared = {
        "risks": RISKS,
        "assurance_states": ASSURANCE_STATES,
        "test_layers": TEST_LAYERS,
        "oracles": ORACLES,
    }
    for key, expected in declared.items():
        actual = set(_unique_strings(allowed[key], f"allowed.{key}", minimum=1, maximum=32))
        if actual != expected:
            raise ReinforcementError(f"allowed.{key} must be exactly {sorted(expected)}")

    matrix_path = _repository_path(root, contract["feature_matrix"], "feature_matrix")
    plan_path = _repository_path(root, contract["plan"], "plan")
    visual_policy_path = _repository_path(root, contract["visual_policy"], "visual_policy")
    if plan_path.suffix.lower() != ".md":
        raise ReinforcementError("feature reinforcement plan must be Markdown")
    _validate_exact_visual_policy(visual_policy_path)

    matrix = _load_json(matrix_path, "feature matrix")
    matrix_features = matrix.get("features") if isinstance(matrix, dict) else None
    if not isinstance(matrix_features, list):
        raise ReinforcementError("feature matrix has no feature list")
    expected_ids = [item.get("id") for item in matrix_features if isinstance(item, dict)]

    features = contract["features"]
    if not isinstance(features, list) or not features:
        raise ReinforcementError("feature test reinforcement features must be non-empty")
    actual_ids = [item.get("id") for item in features if isinstance(item, dict)]
    if actual_ids != expected_ids:
        missing = sorted(set(expected_ids) - set(actual_ids))
        extra = sorted(set(actual_ids) - set(expected_ids))
        raise ReinforcementError(
            f"feature order/coverage differs from feature matrix; missing={missing}, extra={extra}"
        )

    anchors = _markdown_anchors(plan_path)
    ids = set(expected_ids)
    needed_test_count = 0
    owner_count = 0
    for index, raw_feature in enumerate(features):
        feature = _exact_keys(raw_feature, FEATURE_KEYS, f"features[{index}]")
        feature_id = feature["id"]
        if not isinstance(feature_id, str) or not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", feature_id):
            raise ReinforcementError(f"features[{index}].id must be kebab-case")
        if feature["risk"] not in RISKS:
            raise ReinforcementError(f"{feature_id}.risk is unsupported")
        if feature["current_assurance"] not in ASSURANCE_STATES:
            raise ReinforcementError(f"{feature_id}.current_assurance is unsupported")

        plan_anchor = feature["plan_anchor"]
        if plan_anchor != feature_id or plan_anchor not in anchors:
            raise ReinforcementError(f"{feature_id}.plan_anchor must resolve to its exact plan section")

        flags = _exact_keys(feature["flags"], FLAG_KEYS, f"{feature_id}.flags")
        if any(not isinstance(value, bool) for value in flags.values()):
            raise ReinforcementError(f"{feature_id}.flags values must be boolean")

        layers = set(
            _unique_strings(
                feature["test_layers"],
                f"{feature_id}.test_layers",
                minimum=4,
                maximum=len(TEST_LAYERS),
                allowed=TEST_LAYERS,
            )
        )
        oracles = set(
            _unique_strings(
                feature["oracles"],
                f"{feature_id}.oracles",
                minimum=3,
                maximum=len(ORACLES),
                allowed=ORACLES,
            )
        )
        partners = _unique_strings(
            feature["interaction_partners"],
            f"{feature_id}.interaction_partners",
            minimum=1,
            maximum=12,
        )
        for partner in partners:
            if partner not in ids and not partner.startswith("external:"):
                raise ReinforcementError(f"{feature_id} references unknown interaction partner {partner!r}")

        needed_tests = _unique_strings(feature["needed_tests"], f"{feature_id}.needed_tests", minimum=3, maximum=12)
        verification_reinforcements = _unique_strings(
            feature["verification_reinforcements"],
            f"{feature_id}.verification_reinforcements",
            minimum=2,
            maximum=10,
        )
        checker_reinforcements = _unique_strings(
            feature["checker_reinforcements"],
            f"{feature_id}.checker_reinforcements",
            minimum=1,
            maximum=8,
        )
        exit_criteria = _unique_strings(
            feature["exit_criteria"],
            f"{feature_id}.exit_criteria",
            minimum=2,
            maximum=10,
        )
        if not any("negative" in item.lower() or "boundary" in item.lower() or "malformed" in item.lower() for item in needed_tests):
            raise ReinforcementError(f"{feature_id} lacks an explicit negative or boundary test")
        if not any("native" in item.lower() or "platform" in item.lower() or "external" in item.lower() for item in exit_criteria):
            raise ReinforcementError(f"{feature_id} exit criteria do not scope native/external evidence")

        scenario_fields = {
            "needed_tests": needed_tests,
            "verification_reinforcements": verification_reinforcements,
            "checker_reinforcements": checker_reinforcements,
        }
        for field, required_details in REQUIRED_FEATURE_SCENARIO_DETAILS.get(
            feature_id, {}
        ).items():
            combined = " ".join(scenario_fields[field]).casefold()
            for detail in required_details:
                if detail.casefold() not in combined:
                    raise ReinforcementError(
                        f"{feature_id}.{field} is missing required scenario detail {detail!r}"
                    )

        owners = _unique_strings(feature["evidence_owners"], f"{feature_id}.evidence_owners", minimum=1, maximum=12)
        for owner_index, owner in enumerate(owners):
            _repository_path(root, owner, f"{feature_id}.evidence_owners[{owner_index}]")

        if flags["visual"]:
            if not {"visual-pixel", "accessibility"}.issubset(layers):
                raise ReinforcementError(f"{feature_id} visual surface lacks visual and accessibility layers")
            if not {"pixel-exact", "accessibility-tree-event"}.issubset(oracles):
                raise ReinforcementError(f"{feature_id} visual surface lacks exact pixel/accessibility oracles")
        if flags["native"]:
            if "native-e2e" not in layers or "native-process-tree" not in oracles:
                raise ReinforcementError(f"{feature_id} native surface lacks native end-to-end proof")
        if flags["security"]:
            if "property-fuzz" not in layers or "side-effect-absence" not in oracles:
                raise ReinforcementError(f"{feature_id} security boundary lacks fuzz/property and absence proof")
        if flags["persistence"]:
            if "recovery-migration" not in layers or "storage-roundtrip" not in oracles:
                raise ReinforcementError(f"{feature_id} persistence surface lacks recovery/round-trip proof")
        if flags["performance"]:
            if "performance-resource" not in layers or not {"resource-ceiling", "latency-ratchet"}.issubset(oracles):
                raise ReinforcementError(f"{feature_id} performance surface lacks resource and latency proof")

        needed_test_count += len(needed_tests)
        owner_count += len(owners)

    _validate_native_contract_sources(_load_native_contract_sources(root))
    from check_semantic_surfaces import validate_repository as validate_surfaces
    try:
        validate_surfaces(root)
    except ValueError as error:
        raise ReinforcementError(str(error)) from error

    return {
        "features": len(features),
        "needed_tests": needed_test_count,
        "evidence_owners": owner_count,
        "plan_anchors": len(features),
    }


def load_and_validate(path: Path = DEFAULT_CONTRACT, root: Path = ROOT) -> dict[str, int]:
    return validate_document(_load_json(path, "feature test reinforcement contract"), root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    args = parser.parse_args()
    try:
        counts = load_and_validate(args.contract)
    except ReinforcementError as error:
        print(f"feature test reinforcement validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: feature test reinforcement is complete "
        f"(features={counts['features']}, needed_tests={counts['needed_tests']}, "
        f"evidence_owners={counts['evidence_owners']}, plan_anchors={counts['plan_anchors']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
