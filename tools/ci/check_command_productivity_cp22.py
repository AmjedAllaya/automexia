#!/usr/bin/env python3
"""Validate the active CP2.2 Quick Action boundary and safety invariants."""

from __future__ import annotations

import json
import re
from pathlib import Path
import sys
from typing import Any


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


# CP22_SHARED_WORKER_OWNER_V1
WORKER_SOURCE = "apps/automexia-terminal/src/automexia/quick_actions/worker.rs"
WORKER_RUNTIME_SOURCE = "automexia-extension-runtime/src/lib.rs"


def _cp22_worker_code(source: str) -> str:
    """Normalize the reviewed source shape, not arbitrary Rust semantics.

    Supplemental guards only: actual cancellation, TLS, join and deadline
    behavior remain owned by the existing Rust worker/runtime tests. Ignore
    test modules, comments and ordinary literals so they cannot supply missing
    production statements. Structural changes need review, not a fake marker.
    """
    source = source.replace("\r\n", "\n")
    source = re.split(r"\n#\[cfg\(test\)\]\s*\nmod tests\s*\{", source, maxsplit=1)[0]
    source = re.sub(r'/\*.*?\*/|//[^\n]*|"(?:\\.|[^"\\])*"',
                    lambda match: '""' if match[0].startswith('"') else "",
                    source, flags=re.DOTALL)
    source = re.sub(r"\s+", "", source)
    return re.sub(r",(?=[)}])", "", source)


def _cp22_worker_body(source: str, signature: str) -> str:
    if source.count(signature) != 1:
        raise Cp22Error("CP2.2 worker owner signature is missing or ambiguous: " + signature)
    start = source.find("{", source.index(signature) + len(signature))
    if start < 0:
        raise Cp22Error("CP2.2 worker owner body is missing: " + signature)
    depth = 1
    for end in range(start + 1, len(source)):
        if source[end] == "{":
            depth += 1
        elif source[end] == "}":
            depth -= 1
            if depth == 0:
                return source[start + 1:end]
    raise Cp22Error("CP2.2 worker owner body is incomplete: " + signature)


def _validate_worker_lifecycle_sources(worker: str, runtime: str) -> None:
    """Follow application cancellation to the existing shared join owner."""
    app = _cp22_worker_code(worker)
    shared = _cp22_worker_code(runtime)

    def require(condition: bool, message: str) -> None:
        if not condition:
            raise Cp22Error("CP2.2 shared worker ownership: " + message)

    body = _cp22_worker_body
    inner = body(app, "structRuntimeInner")
    require("worker:Option<BoundedWorker<WorkerRun>>" in inner,
            "the application must retain its BoundedWorker owner")
    # Path::join(path) and slice::join(separator) are unrelated to thread
    # cleanup. Only reject the native handle call shapes in this reviewed
    # source; retain the exact owner/delegation/body checks below. This is a
    # supplemental source guard, not Rust type resolution or runtime proof.
    require(".join()" not in app and not re.search(
        r"\b(?:JoinHandle|ScopedJoinHandle)(?:(?:::)?<[^;{}]*>)?::join\b", app),
        "native thread join must remain with the shared cleanup owner")
    spawn = body(app, "fnspawn_worker(")
    require('letworker=BoundedWorker::new("",1,run_worker);' in spawn
            and "(worker.try_submit(run)==RefreshSubmission::Queued).then_some(worker)" in spawn,
            "admission must use the existing capacity-one shared worker")
    cancel = body(body(app, "implRuntimeInner"), "fnrequest_shutdown(&self)")
    expected = (
        "{letmutstate=lock(&self.pending.0);state.shutdown=true;"
        "lock(&self.latest_requested).clear();lock(&self.results).clear();"
        "lock(&self.workspace_authorizations).clear();lock(&self.provider_snapshots).clear();}"
        "self.pending.1.notify_all();"
        "ifletSome(worker)=&self.worker{worker.request_shutdown();}"
    )
    require(cancel == expected, "cancel publication and wake before requesting shared retirement")
    app_drop = body(body(app, "implDropforRuntimeInner"), "fndrop(&mutself)")
    require(app_drop == "self.request_shutdown();", "application Drop must request retirement without joining")
    public = body(app, "implQuickActionRuntime")
    require(body(public, "pubfnrequest_shutdown(&self)") == "self.0.request_shutdown();",
            "the public cancellation API must delegate to its owner")
    wait = body(public, "pubfnshutdown_timeout(&self,timeout:Duration)->bool")
    require(wait == (
        "letstarted=Instant::now();self.request_shutdown();"
        "self.0.worker.as_ref().is_none_or(|worker|{"
        "worker.shutdown_timeout(timeout.saturating_sub(started.elapsed()))})"
    ), "wait must return shared acknowledgement within the remaining caller budget")

    control = body(body(shared, "impl<T>WorkerThread<T>"), "fnrequest_shutdown(&self)")
    require(control == "self.stopping.store(true,Ordering::Release);"
                       "let_=self.sender.try_send(WorkerMessage::Shutdown);",
            "shared cancellation must not block on full queue capacity")
    cleanup = body(body(shared, "implCleanupService"), "fnnew(name:&str,permit:OwnerPermit)->Option<Self>")
    joined = ("matchjob.handle.join(){Ok(())=>job.completion.finish(),"
              "Err(payload)=>{job.completion.fail();std::panic::resume_unwind(payload);}}")
    require(joined in cleanup and cleanup.count("job.handle.join()") == 1
            and cleanup.count("job.completion.finish()") == 1,
            "cleanup must acknowledge only successful native join and retain failure handling")
    require(".is_finished(" not in shared, "a finished-function hint is not join acknowledgement")
    complete = body(body(shared, "implJoinCompletion"), "fncomplete(&self)->bool")
    require(complete == "self.finished.load(Ordering::Acquire)"
                        "&&self.registrations.load(Ordering::Acquire)==0"
                        "&&!self.failed.load(Ordering::Acquire)",
            "completion must include registration retirement and exclude cleanup failure")
    implementation = body(shared, "impl<T>BoundedWorker<T>")
    native_cancel = body(implementation,
                        '#[cfg(not(target_arch=""))]pubfnrequest_shutdown(&self)')
    require(native_cancel == "ifletSome(worker)=self.lock_slot().worker.as_ref(){worker.request_shutdown();}",
            "the public shared cancellation API must reach its worker")
    status = body(implementation,
                  '#[cfg(not(target_arch=""))]pubfnshutdown_status(&self)->WorkerShutdownStatus')
    require(status == (
        "letslot=self.lock_slot();letSome(worker)=slot.worker.as_ref()else{returnWorkerShutdownStatus::Complete;};"
        "ifworker.completion.failed.load(Ordering::Acquire){WorkerShutdownStatus::CleanupFailed}"
        "elseifworker.completion.complete(){WorkerShutdownStatus::Complete}"
        "elseifworker.stopping.load(Ordering::Acquire)||worker.completion.finished.load(Ordering::Acquire)"
        "{WorkerShutdownStatus::Retiring}else{WorkerShutdownStatus::Running}"
    ), "status must distinguish failed, retiring and actually complete cleanup")
    native_wait = body(implementation,
                      '#[cfg(not(target_arch=""))]pubfnshutdown_timeout(&self,timeout:Duration)->bool')
    require(native_wait == (
        "letcompletion={letslot=self.lock_slot();letSome(worker)=slot.worker.as_ref()else{returntrue;};"
        "worker.request_shutdown();Arc::clone(&worker.completion)};completion.wait(timeout)"
    ), "the shared wait must release its slot lock and return the bounded completion result")
    ensure = body(implementation, "fnensure_thread(&self,slot:&mutWorkerSlot<T>)->bool")
    require("cleanup.own(job);slot.worker=Some(worker);true" in ensure,
            "the cleanup service must own the join job before worker publication")
    require("ifslot.worker.as_ref().is_some_and(|worker|worker.completion.complete()){slot.worker=None;}" in ensure,
            "replacement must wait for the prior generation's acknowledged completion")
    shared_drop = body(body(shared, "impl<T>DropforBoundedWorker<T>"), "fndrop(&mutself)")
    require(shared_drop == "self.request_shutdown();", "BoundedWorker Drop must retain asynchronous cleanup ownership")


def validate_worker_lifecycle() -> None:
    _validate_worker_lifecycle_sources(
        bounded_text(ROOT / WORKER_SOURCE),
        bounded_text(ROOT / WORKER_RUNTIME_SOURCE),
    )


def validate_sources(document: dict[str, Any]) -> dict[str, int]:
    validate_worker_lifecycle()
    for relative in document["model_files"]:
        source = bounded_text(ROOT / relative).casefold()
        marker = next((item for item in sorted(MODEL_FORBIDDEN) if item in source), None)
        if marker:
            raise Cp22Error(f"{relative} crosses the capability-free model boundary: {marker}")

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
            "self.paste(&expanded.command, true)",
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
