#!/usr/bin/env python3
"""Validate the active CP2.2 Quick Action boundary and safety invariants."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any

from check_command_productivity import (
    rust_code_without_comments_and_literals,
    validate_interactive_grid_boundary,
    CommandProductivityError,
)


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/command-productivity/cp22-contract-v1.json"
MAX_POLICY_BYTES = 262_144
MAX_GRID_OWNER_BYTES = 393_216
EXPECTED_LIMITS = {
    "source_file_bytes": 1_048_576,
    "resident_cache_bytes": 8_388_608,
    "active_actions": 1_024,
    "query_bytes": 4_096,
    "search_results": 128,
    "expanded_command_bytes": 65_536,
    "result_routes": 32,
    "search_coalesce_ms": 12,
    "watcher_events": 64,
}
EXPECTED_PRECEDENCE = [
    "session",
    "capsule",
    "trusted-workspace",
    "shell-user",
    "global-user",
    "builtin-disabled",
]
EXPECTED_EXECUTION = {
    "insert": "explicit-reviewed-bracketed-paste-without-enter",
    "copy": "explicit-reviewed-clipboard-write",
    "exact_launch": "disabled-until-D3",
}
EXPECTED_CAPABILITIES = {
    "network": False,
    "provider_process": False,
    "environment_read_in_model": False,
    "terminal_grid_inference": False,
    "implicit_execution": False,
    "exact_launch": False,
    "secret_read": False,
    "clipboard_write_after_review": True,
    "pty_insert_after_review": True,
}
MODEL_FORBIDDEN = {
    "std::env",
    "std::fs",
    "std::net",
    "std::process",
    "clipboard",
    "notify::",
    "rio_vt::",
    "teletypewriter::",
    "automexia_ui_model::",
    "unsafe {",
}
EXPECTED_OWNERSHIP = {
    "model": "automexia-command-productivity-capability-free",
    "persistence_and_worker": "automexia-terminal-application",
    "view_model": "automexia-ui-model-renderer-independent",
    "renderer_and_input": "automexia-terminal-frontend",
}
WORKER_FORBIDDEN = {
    "std::env",
    "std::net",
    "std::process",
    "clipboard",
    "terminal.grid",
    "teletypewriter::",
    "unsafe {",
}
ACTION_SURFACE_FORBIDDEN = WORKER_FORBIDDEN - {"clipboard"}


class Cp22Error(ValueError):
    pass


def bounded_text(path: Path, maximum: int = MAX_POLICY_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise Cp22Error(f"required regular file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise Cp22Error(f"policy source exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise Cp22Error(f"policy source grew while reading: {path}")
    return data.decode("utf-8")


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path))
    if not isinstance(document, dict):
        raise Cp22Error("CP2.2 contract must be an object")
    expected_keys = {
        "schema",
        "phase",
        "status",
        "ownership",
        "limits",
        "scope_precedence",
        "workspace_activation",
        "secret_expansion",
        "execution",
        "platforms",
        "capabilities",
        "model_files",
        "application_files",
        "ui_files",
        "required_tests",
        "benchmark",
    }
    if set(document) != expected_keys:
        raise Cp22Error("CP2.2 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1,
        "CP2.2",
        "active",
    ):
        raise Cp22Error("CP2.2 contract identity must remain active schema 1")
    if document["limits"] != EXPECTED_LIMITS:
        raise Cp22Error("CP2.2 resource ceilings changed")
    if document["ownership"] != EXPECTED_OWNERSHIP:
        raise Cp22Error("CP2.2 ownership boundary changed")
    if document["scope_precedence"] != EXPECTED_PRECEDENCE:
        raise Cp22Error("CP2.2 scope precedence changed")
    if document["execution"] != EXPECTED_EXECUTION:
        raise Cp22Error("CP2.2 execution policy changed")
    if document["capabilities"] != EXPECTED_CAPABILITIES:
        raise Cp22Error("CP2.2 capability boundary changed")
    if document["workspace_activation"] != "disabled-until-exact-trust":
        raise Cp22Error("workspace Quick Actions must remain disabled")
    if document["secret_expansion"] != "disabled-until-secret-broker":
        raise Cp22Error("secret expansion must remain disabled")
    for key in ("model_files", "application_files", "ui_files", "required_tests"):
        values = document[key]
        if not isinstance(values, list) or not values or len(values) != len(set(values)):
            raise Cp22Error(f"{key} must contain unique non-empty entries")
    return document


def require_tokens(relative: str, tokens: set[str]) -> str:
    source = bounded_text(ROOT / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise Cp22Error(f"{relative} is missing CP2.2 evidence: {missing}")
    return source


QUICK_ACTION_WORKER = "apps/automexia-terminal/src/automexia/quick_actions/worker.rs"
EXTENSION_WORKER = "automexia-extension-runtime/src/lib.rs"
ROUTER = "apps/automexia-terminal/src/router/mod.rs"


def _rust_item(source: str, declaration: str) -> str:
    """Extract one known item; ambiguous or incomplete declarations fail closed."""
    if source.count(declaration) != 1:
        raise Cp22Error(f"Quick Action lifecycle declaration is ambiguous: {declaration}")
    start = source.find("{", source.index(declaration) + len(declaration))
    if start < 0:
        raise Cp22Error(f"Quick Action lifecycle item has no body: {declaration}")
    depth = 1
    for end in range(start + 1, len(source)):
        depth += (source[end] == "{") - (source[end] == "}")
        if depth == 0:
            return source[start + 1:end]
    raise Cp22Error(f"Quick Action lifecycle item is incomplete: {declaration}")


def _compact(source: str) -> str:
    return "".join(source.split())


def _lifecycle_fragment(body: str, fragment: str, owner: str) -> None:
    if _compact(fragment) not in _compact(body):
        raise Cp22Error(f"Quick Action lifecycle is disconnected: {owner}")


def _lifecycle_body(body: str, expected: str, owner: str) -> None:
    if _compact(body) != _compact(expected):
        raise Cp22Error(f"Quick Action lifecycle adapter changed: {owner}")


def validate_worker_lifecycle(root: Path | None = None) -> None:
    """Ratchet ADR 0076's application-to-existing-cleanup-owner handoff.

    The generic retirement gate and native worker tests remain authoritative for
    admission ceilings, registration lifetime and actual join acknowledgement.
    This gate connects the Quick Action owner to that implementation and rejects
    proof left only in comments, tests, or unrelated helper functions.
    """
    root = ROOT if root is None else root
    worker = rust_code_without_comments_and_literals(bounded_text(root / QUICK_ACTION_WORKER))
    runtime = _rust_item(worker, "impl QuickActionRuntime")
    inner = _rust_item(worker, "impl RuntimeInner")
    _lifecycle_fragment(
        _rust_item(worker, "struct RuntimeInner"),
        "worker: Option<BoundedWorker<WorkerRun>>", "application worker field",
    )
    opened = _rust_item(runtime, "pub fn open(")
    running = _rust_item(worker, "fn run_worker(")
    for fragment in (
        "let worker = spawn_worker(",
        "WorkerShared { pending: Arc::clone(&pending),",
        ").ok_or(QuickActionRuntimeErrorCode::WorkerUnavailable)?;",
        "Ok(Self(Arc::new(RuntimeInner {",
        "worker: Some(worker),",
    ):
        _lifecycle_fragment(opened, fragment, "application worker admission")
    spawned = _rust_item(worker, "fn spawn_worker(")
    for fragment in (
        "let worker = BoundedWorker::new(, 1, run_worker);",
        "let pending_cleanup = PendingCleanup { pending: Arc::clone(&shared.pending), };",
        "let run = WorkerRun { monitor, index, shared, _pending_cleanup: pending_cleanup, };",
        "(worker.try_submit(run) == RefreshSubmission::Queued).then_some(worker)",
    ):
        _lifecycle_fragment(spawned, fragment, "capacity-one kickoff ownership")
    for fragment in (
        "let WorkerRun { mut monitor, mut index, shared, _pending_cleanup, } = run;",
        "if lock(&pending.0).shutdown { break; }",
        "std::mem::take(&mut state.latest_by_route)",
    ):
        _lifecycle_fragment(running, fragment, "worker-owned queued callbacks")
    if _compact(running).count("_pending_cleanup") != 1:
        raise Cp22Error("Quick Action lifecycle guard no longer stays with the worker")
    _lifecycle_fragment(
        _rust_item(_rust_item(worker, "impl Drop for PendingCleanup"), "fn drop("),
        "let queued = std::mem::take(&mut lock(&self.pending.0).latest_by_route); drop(queued);",
        "queued callback drain guard",
    )
    cancelled = _rust_item(inner, "fn request_shutdown(")
    _lifecycle_body(cancelled, """
        { let mut state = lock(&self.pending.0);
          state.shutdown = true;
          state.mutation_routes.clear(); state.mutation_results.clear();
          lock(&self.latest_requested).clear(); lock(&self.results).clear();
          lock(&self.workspace_authorizations).clear(); lock(&self.provider_snapshots).clear(); }
        self.pending.1.notify_all();
        if let Some(worker) = &self.worker { worker.request_shutdown(); }
    """, "serialized cancellation before worker retirement")
    if any(token in cancelled for token in ("latest_by_route", "shutdown_timeout", ".join(")):
        raise Cp22Error("Quick Action cancellation moved foreign cleanup onto the caller")
    dropped = _rust_item(_rust_item(worker, "impl Drop for RuntimeInner"), "fn drop(")
    if _compact(dropped) != "self.request_shutdown();":
        raise Cp22Error("Quick Action Drop must only request owned retirement")
    _lifecycle_body(
        _rust_item(runtime, "pub fn request_shutdown("),
        "self.0.request_shutdown();", "public cancellation owner",
    )
    _lifecycle_body(_rust_item(runtime, "pub fn shutdown_timeout("), """
        let started = Instant::now(); self.request_shutdown();
        self.0.worker.as_ref().is_none_or(|worker| {
            worker.shutdown_timeout(timeout.saturating_sub(started.elapsed()))
        })
    """, "one-budget native cleanup acknowledgement")

    # Follow this worker's handle and completion into the existing generic owner;
    # do not add an application join registry or copy runtime retirement logic.
    extension = rust_code_without_comments_and_literals(bounded_text(root / EXTENSION_WORKER))
    bounded = _rust_item(extension, "impl<T> BoundedWorker<T>")
    _lifecycle_fragment(_rust_item(bounded, "fn ensure_thread("), """
        let Some(cleanup) = slot.cleanup.as_ref() else { return false; };
        let Some((worker, job)) = self.spawn_thread() else { return false; };
        cleanup.own(job); slot.worker = Some(worker); true
    """, "registered native worker cleanup handoff")
    _lifecycle_fragment(_rust_item(bounded, "fn spawn_thread("), """
        let job = JoinJob { handle, completion: Arc::clone(&completion), };
        Some((WorkerThread { sender, stopping, completion, }, job,))
    """, "same native handle and completion ownership")
    cleanup = _rust_item(extension, "impl CleanupService")
    _lifecycle_fragment(_rust_item(cleanup, "fn own("), """
        let mut pending = self.mailbox.job.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    """, "sole cleanup mailbox")
    _lifecycle_fragment(_rust_item(cleanup, "fn own("), """
        *pending = Some(job); self.mailbox.changed.notify_one();
    """, "native job publication before cleanup wake")
    joined = _rust_item(cleanup, "fn new(")
    for fragment in (
        "let worker_mailbox = Arc::clone(&mailbox);",
        "spawn(move || { let _permit = permit;",
        "let guard = worker_mailbox.job.lock()",
        "guard.take()",
        "let Some(job) = job else { break };",
        "match job.handle.join() { Ok(()) => job.completion.finish(),",
        "Some(Self { mailbox, _handle: handle, })",
    ):
        _lifecycle_fragment(joined, fragment, "worker-local native cleanup owner")
    if _compact(joined).count("job.completion.finish()") != 1:
        raise Cp22Error("Quick Action lifecycle cleanup acknowledgement precedes native cleanup")
    router = rust_code_without_comments_and_literals(bounded_text(root / ROUTER))
    _lifecycle_fragment(
        _rust_item(router, "pub fn shutdown_services("),
        "self.quick_actions.request_shutdown();", "application shutdown cancellation",
    )

def validate_workflow_boundary(root: Path | None = None) -> None:
    root = ROOT if root is None else root
    def code(path: str) -> str:
        return rust_code_without_comments_and_literals(bounded_text(root / path))
    model_path = "automexia-command-productivity/src/actions/workflow.rs"
    model = code(model_path)
    for marker in MODEL_FORBIDDEN:
        if marker in model.casefold():
            raise Cp22Error(f"workflow model acquired a capability: {marker}")
    for fragment in ("MAX_WORKFLOW_STEPS: usize = 32;", "!(1..=3600).contains(&step.timeout_seconds)"):
        _lifecycle_fragment(model, fragment, "bounded workflow model")
    poll = _rust_item(model, "pub fn poll(")
    for fragment in ("observed.generation != self.baseline.generation", "observed.input_revision != self.baseline.input_revision",
                     "let Some((source, status)) = observed.completed else", "if status != 0", "Some(source) != self.baseline.prompt"):
        _lifecycle_fragment(poll, fragment, "verified workflow progression")
    surface = code("apps/automexia-terminal/src/screen/action_surface/workflow.rs")
    start = _rust_item(surface, "pub(super) fn workflow_control(")
    for fragment in ("if !self.ensure_action_scope_authorized()", "if !unchanged", "a == action.as_ref() && a.enabled",
                     "self.invalidate_workflow_submission();", "workflow.finished = true"):
        _lifecycle_fragment(start, fragment, "review, cancellation and source validity")
    synchronize = _rust_item(surface, "pub(super) fn sync_action_workflow(")
    for fragment in ("context.route_id != workflow.route", "context.rich_text_id != workflow.rich_text", "deliver_workflow_command(target, &step.command, receipt)"):
        _lifecycle_fragment(synchronize, fragment, "route-owned submission")
    terminal = code("rio-vt/src/crosswords/command_actions.rs")
    accept = _rust_item(terminal, "pub fn accept_workflow_submission(")
    for fragment in ("self.workflow_prompt() != Some(receipt)", "self.workflow_input.submitted == Some(receipt)", "self.workflow_input.submitted = Some(receipt)"):
        _lifecycle_fragment(accept, fragment, "one-use prompt receipt")
    performer = code("rio-vt/src/performer/mod.rs")
    resolve = _rust_item(performer, "fn resolve_pending_paste(")
    for fragment in ("accept_workflow_submission(receipt)", "encode_paste(paste, terminal.mode())"):
        _lifecycle_fragment(resolve, fragment, "PTY writer revalidation")
    # Keep the revalidation after the bounded drain; comments/dead helpers do
    # not count as evidence of the live submission path.
    if resolve.find("drain") > resolve.find("accept_workflow_submission") or "drain" not in resolve:
        raise Cp22Error("workflow submission must follow output drain")
    request = code("rio-vt/src/event/mod.rs")
    normal = _rust_item(_rust_item(request, "impl PasteRequest"), "pub fn new(")
    _lifecycle_fragment(normal, "reviewed_submission: None", "ordinary paste cannot execute")
    _lifecycle_fragment(normal, "reviewed_command: false", "ordinary paste encoding unchanged")
    insert = _rust_item(request, "pub fn reviewed_action_insert(")
    _lifecycle_fragment(insert, "reviewed_submission: None", "ordinary action cannot execute")
    worker = code(QUICK_ACTION_WORKER)
    submit = _rust_item(worker, "pub fn submit_mutation(")
    for fragment in ("pending.shutdown", "pending.mutation_routes.contains_key(&route_id)", "pending.mutation_routes.len() >= MAX_RESULT_ROUTES", "pending.mutations.len() >= 8"):
        _lifecycle_fragment(submit, fragment, "bounded editor admission")
    cleanup = _rust_item(_rust_item(worker, "impl Drop for PendingCleanup"), "fn drop(")
    _lifecycle_fragment(cleanup, "let mutations = std::mem::take(&mut lock(&self.pending.0).mutations); drop(mutations);", "worker-owned mutation cleanup")


def validate_sources(document: dict[str, Any]) -> dict[str, int]:
    for relative in document["model_files"]:
        source = bounded_text(ROOT / relative).casefold()
        marker = next((item for item in sorted(MODEL_FORBIDDEN) if item in source), None)
        if marker:
            raise Cp22Error(f"{relative} crosses the capability-free model boundary: {marker}")

    validate_worker_lifecycle()
    validate_workflow_boundary()
    worker = require_tokens(
        "apps/automexia-terminal/src/automexia/quick_actions/worker.rs",
        {
            "SEARCH_COALESCE_INTERVAL",
            "MAX_RESULT_ROUTES",
            "latest_requested",
            "latest_by_route",
            "RouteCapacity",
            "validate_search_query",
            "forget_route",
        },
    ).casefold()
    marker = next((item for item in sorted(WORKER_FORBIDDEN) if item in worker), None)
    if marker:
        raise Cp22Error(f"Quick Action worker crosses its capability boundary: {marker}")

    require_tokens(
        "automexia-command-productivity/src/actions/activation.rs",
        {
            "MAX_SEARCH_RESULTS",
            "MAX_EXPANDED_COMMAND_BYTES",
            "ExactLaunchDisabled",
            "SecretReferenceUnavailable",
            "workspace_trusted",
            "LayerIdentity::ShellUser",
            "LayerIdentity::GlobalUser",
            "validate_quick_actions",
        },
    )
    require_tokens(
        "apps/automexia-terminal/src/automexia/quick_actions/transfer.rs",
        {
            "MAX_SOURCE_BYTES",
            "source_digest",
            "replace_conflicts",
            "allow_machine_paths",
            "sync_directory",
        },
    )
    surface = require_tokens(
        "apps/automexia-terminal/src/screen/action_surface.rs",
        {
            ".deliver_action_insert(target, &expanded.command)",
            "requires_second_confirmation",
            "submit_action_placeholder",
            "unavailable_before_placeholder",
            "SecretReference",
            "action_notice",
            "workspace_trusted: false",
        },
    ).casefold()
    marker = next(
        (item for item in sorted(ACTION_SURFACE_FORBIDDEN) if item in surface), None
    )
    if marker:
        raise Cp22Error(f"action-surface adapter crosses its capability boundary: {marker}")
    require_tokens(
        "apps/automexia-terminal/src/renderer/command_palette.rs",
        {
            "QuickActionPlaceholder",
            "QuickActionReview",
            "Exact command",
            "MAX_PALETTE_QUERY_BYTES",
            "QuickActionNotice",
            "metadata_label",
        },
    )
    require_tokens(
        "apps/automexia-terminal/src/cli.rs",
        {"ActionsAction", "expected_revision", "requires = \"apply\""},
    )
    require_tokens(
        document["benchmark"],
        {"quick_action_search_1024", "quick_action_expand_and_quote"},
    )
    screen = bounded_text(ROOT / "apps/automexia-terminal/src/screen/action_surface.rs")
    if "send_write(expanded.command" in screen or "expanded.command.push('\\r')" in screen:
        raise Cp22Error("reviewed Quick Actions must never synthesize Enter")
    grid_owner = bounded_text(
        ROOT / "apps/automexia-terminal/src/screen/mod.rs", MAX_GRID_OWNER_BYTES
    ).casefold()
    try:
        validate_interactive_grid_boundary("apps/automexia-terminal/src/screen/mod.rs", grid_owner)
    except CommandProductivityError as error:
        raise Cp22Error(str(error)) from error
    grid_owner = rust_code_without_comments_and_literals(grid_owner)
    dispatch = _rust_item(grid_owner, "pub fn activate_palette_selection(")
    # Only the reviewed composition-root dispatcher may delegate to the owner.
    # Its body was checked above for grid inference; domain logic stays separate.
    grid_owner = grid_owner.replace(dispatch, "", 1)
    if "quickaction" in grid_owner or "quick_action" in grid_owner or "quick action" in grid_owner:
        raise Cp22Error("grid-owning screen module must not contain Quick Action domain logic")
    return {
        "model_files": len(document["model_files"]),
        "application_files": len(document["application_files"]),
        "ui_files": len(document["ui_files"]),
        "tests": len(document["required_tests"]),
    }


def validate_documents() -> None:
    for relative, tokens in {
        "docs/COMMAND-PRODUCTIVITY.md": {"CP2.2", "actions import", "Insert without Enter"},
        "docs/DEVOPS-ALIASES.md": {"CP2.2", "Quick Actions", "exact launch"},
        "docs/TESTING.md": {"CP2.2", "quick_action_search_1024"},
        "docs/PHASE-IMPLEMENTATION-AUDIT.md": {
            "Command productivity foundations",
            "Feature documentation and source tests remain authoritative",
        },
    }.items():
        require_tokens(relative, tokens)


def validate_repository() -> dict[str, int]:
    document = load_contract()
    counts = validate_sources(document)
    validate_documents()
    return counts


def main() -> int:
    try:
        counts = validate_repository()
    except (Cp22Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"CP2.2 Quick Action validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: CP2.2 Quick Actions preserve bounded model/app/UI ownership, "
        "review-before-insert, and disabled exact/secret/workspace authority "
        f"({counts})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
