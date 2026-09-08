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

NATIVE_CONTRACT_SOURCES = {
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


def _validate_native_contract_sources(sources: dict[str, str]) -> None:
    missing = set(NATIVE_CONTRACT_SOURCES) - set(sources)
    if missing:
        raise ReinforcementError(f"native assurance sources are missing {sorted(missing)}")

    _validate_shortcut_editor_sources(sources)
    _validate_child_exit_sources(sources)

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
