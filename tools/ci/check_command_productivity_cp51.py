#!/usr/bin/env python3
"""Validate the non-activating CP5.1-CP5.6 editor-bridge proposal."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = Path(
    "tests/fixtures/command-productivity/cp51-bridge-threat-contract-v1.json"
)
ADR_PATH = Path(
    "docs/adr/0025-authenticated-native-editor-suggestion-bridge.md"
)
PLAN_PATH = Path("docs/research/CP51-CP56-IMPLEMENTATION-AUDIT.md")
MAX_CONTRACT_BYTES = 131_072
MAX_DOCUMENT_BYTES = 262_144
EXPECTED_CANONICAL_SHA256 = (
    "d8587db7d5f600dc55718d39a98005afa4c16d3cae8f3ed4348306011d9b948c"
)
EXPECTED_TOP_LEVEL_KEYS = {
    "schema",
    "phase",
    "status",
    "authority",
    "transport",
    "identity_binding",
    "framing",
    "request",
    "candidate",
    "security_threats",
    "sources",
    "ranking",
    "ui",
    "shell_matrix",
    "limits",
    "lifecycle",
    "verification",
    "external_gates",
}
EXPECTED_THREATS = {
    "CP5-T17-endpoint-impersonation-replay",
    "CP5-T18-buffer-history-privacy",
    "CP5-T19-stale-overbroad-replacement",
    "CP5-T20-candidate-spoofing",
    "CP5-T21-input-capture-occlusion",
    "CP5-T22-resource-amplification",
}
EXPECTED_SOURCES = [
    "native-shell",
    "opt-in-shell-history",
    "shell-cwd-and-executables",
    "opt-in-decayed-frequency",
    "cached-cp1-cp4-public",
    "typed-cp2-cp3-actions",
]
EXPECTED_RANKING = [
    "exact-prefix",
    "native-rank",
    "word-boundary-prefix",
    "case-insensitive-prefix",
    "opt-in-frequency",
    "fuzzy-score",
]
EXPECTED_SHELLS = {
    "powershell-7",
    "windows-powershell-5.1",
    "bash",
    "zsh",
    "fish",
    "cmd",
    "wsl",
}
EXPECTED_LIMITS = {
    "buffer_bytes": 16_384,
    "frame_bytes": 1_048_576,
    "candidate_count": 512,
    "candidate_bytes": 1_024,
    "batch_bytes": 524_288,
    "visible_rows": 12,
    "per_pane_queued_requests": 1,
    "active_routes": 64,
    "cache_bytes": 8_388_608,
    "source_deadline_ms": 250,
    "warm_local_p95_ms": 50,
    "render_p95_ms": 8,
    "cancellation_p95_ms": 50,
    "announcement_coalesce_ms": 250,
    "optional_animation_ms": 120,
}


class Cp51Error(ValueError):
    """The CP5.1-CP5.6 proposal contract is invalid."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    document: dict[str, Any] = {}
    for key, value in pairs:
        if key in document:
            raise Cp51Error(f"CP5.1 contract contains duplicate key {key!r}")
        document[key] = value
    return document


def parse_contract(text: str) -> Any:
    return json.loads(text, object_pairs_hook=_unique_object)


def _bounded_text(path: Path, maximum: int, label: str) -> str:
    if path.is_symlink():
        raise Cp51Error(f"{label} must not be a symbolic link: {path}")
    if not path.is_file():
        raise Cp51Error(f"{label} is missing: {path}")
    if path.stat().st_size > maximum:
        raise Cp51Error(f"{label} exceeds {maximum} bytes: {path}")
    return path.read_text(encoding="utf-8")


def _require_exact_keys(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise Cp51Error(f"{label} keys changed")
    return value


def validate_contract(document: Any) -> dict[str, int]:
    document = _require_exact_keys(document, EXPECTED_TOP_LEVEL_KEYS, "contract")
    if (document["schema"], document["phase"], document["status"]) != (
        1,
        "CP5.1-CP5.6",
        "proposed-not-authorized",
    ):
        raise Cp51Error("proposal status or schema changed")

    authority = _require_exact_keys(
        document["authority"],
        {
            "adr",
            "accepted",
            "runtime_activation",
            "default_experience",
            "fallback",
            "dependency",
            "insertion_owner",
            "execution",
        },
        "authority",
    )
    if authority != {
        "adr": "0025",
        "accepted": False,
        "runtime_activation": False,
        "default_experience": "cp1-shell-native",
        "fallback": "native-editor",
        "dependency": "none",
        "insertion_owner": "native-shell-editor",
        "execution": "never",
    }:
        raise Cp51Error("proposal gained authority or changed ownership")

    transport = _require_exact_keys(
        document["transport"], {"windows", "unix", "forbidden"}, "transport"
    )
    if transport["windows"] != {
        "kind": "private-named-pipe",
        "remote_clients": "rejected",
        "first_instance": True,
        "acl": "current-user-and-logon-session-only",
        "peer_checks": ["client-process-id", "client-session-id"],
    }:
        raise Cp51Error("Windows endpoint controls changed")
    if transport["unix"] != {
        "kind": "filesystem-unix-stream-socket",
        "directory_mode": "0700",
        "socket_mode": "0600",
        "abstract_namespace": False,
        "peer_checks": ["effective-user-id"],
    }:
        raise Cp51Error("Unix endpoint controls changed")
    if transport["forbidden"] != [
        "tcp",
        "udp",
        "osc",
        "terminal-output",
        "terminal-grid-inference",
        "per-keystroke-process",
    ]:
        raise Cp51Error("forbidden transports changed")

    identity = document["identity_binding"]
    required_identity = [
        "application-generation",
        "window",
        "tab",
        "pane",
        "session",
        "shell",
        "editor-version",
        "endpoint-instance",
        "capability",
        "prompt-generation",
        "buffer-generation",
    ]
    if identity.get("required") != required_identity:
        raise Cp51Error("route identity binding changed")
    if identity.get("capability_bytes") != 32 or identity.get("replay_window") != 0:
        raise Cp51Error("capability or replay policy changed")
    if identity.get("compare") != "constant-time" or identity.get("rotation") != [
        "session-start",
        "route-rebind",
        "kill-switch",
    ]:
        raise Cp51Error("capability comparison or rotation changed")

    framing = document["framing"]
    if framing != {
        "encoding": "utf-8-json",
        "length_prefix": "u32-little-endian",
        "schema_negotiation": "exact-major-overlapping-minor",
        "allocation_rule": "validate-prefix-before-allocation",
        "unknown_fields": "reject",
        "compression": "forbidden",
    }:
        raise Cp51Error("framing policy changed")

    request = document["request"]
    required_request = {
        "schema",
        "request_id",
        "application_generation",
        "window_id",
        "tab_id",
        "pane_id",
        "session_id",
        "shell",
        "editor_version",
        "endpoint_instance",
        "capability",
        "prompt_generation",
        "buffer_generation",
        "buffer",
        "cursor_byte",
        "cursor_grapheme",
        "selection",
        "replacement_span",
        "quote_context",
        "token_context",
        "cwd",
        "completion_mode",
        "source_revision",
        "reason",
        "cancellation_id",
    }
    if set(request.get("required_fields", [])) != required_request:
        raise Cp51Error("request binding fields changed")
    if request.get("persistence") != "memory-only-drop-after-response":
        raise Cp51Error("request privacy changed")
    if request.get("logging") != "metadata-allowlist-only":
        raise Cp51Error("request logging changed")

    candidate = document["candidate"]
    required_candidate = {
        "request_id",
        "candidate_id",
        "insertion",
        "display",
        "description",
        "kind",
        "source",
        "freshness",
        "replacement_span",
        "quoting",
        "public_context",
        "risk",
    }
    if set(candidate.get("required_fields", [])) != required_candidate:
        raise Cp51Error("candidate fields changed")
    if candidate.get("id_scope") != "request-local-stable":
        raise Cp51Error("candidate identity changed")
    if candidate.get("acceptance") != "editor-revalidate-replace-once-no-enter":
        raise Cp51Error("candidate acceptance changed")
    if candidate.get("display_policy") != "plain-text-bidi-contained-grapheme-safe":
        raise Cp51Error("candidate display policy changed")

    threats = document["security_threats"]
    if not isinstance(threats, list) or len(threats) != len(EXPECTED_THREATS):
        raise Cp51Error("security threat count changed")
    ids = {item.get("id") for item in threats if isinstance(item, dict)}
    if ids != EXPECTED_THREATS:
        raise Cp51Error("security threat IDs changed")
    threat_keys = {
        "id",
        "asset",
        "controls",
        "hostile_mutations",
        "verification_owner",
        "residual_risk",
    }
    for threat in threats:
        if set(threat) != threat_keys:
            raise Cp51Error(f"threat {threat.get('id')} keys changed")
        for key in ("controls", "hostile_mutations", "verification_owner"):
            if not isinstance(threat[key], list) or not threat[key]:
                raise Cp51Error(f"threat {threat['id']} lacks {key}")
        if not isinstance(threat["residual_risk"], str) or not threat["residual_risk"]:
            raise Cp51Error(f"threat {threat['id']} lacks residual risk")

    sources = document["sources"]
    if [source.get("id") for source in sources] != EXPECTED_SOURCES:
        raise Cp51Error("source priority changed")
    for source in sources:
        if set(source) != {"id", "priority", "opt_in", "owner", "io"}:
            raise Cp51Error(f"source {source.get('id')} keys changed")
        if source["io"] not in {"none", "shell-local-nonrecursive", "cached-memory"}:
            raise Cp51Error(f"source {source['id']} gained unreviewed IO")
    if document["ranking"].get("order") != EXPECTED_RANKING:
        raise Cp51Error("ranking order changed")
    if document["ranking"].get("tie_breakers") != [
        "source-priority",
        "normalized-display",
        "candidate-id",
    ]:
        raise Cp51Error("ranking tie breakers changed")

    ui = document["ui"]
    if ui.get("owner") != "one-pane-immutable-renderer-neutral-snapshot":
        raise Cp51Error("UI ownership changed")
    if ui.get("semantics") != ["listbox", "option", "selected", "position", "set-size"]:
        raise Cp51Error("UI semantics changed")
    if ui.get("dismiss_on") != [
        "focus-loss",
        "buffer-change",
        "generation-change",
        "pane-change",
        "prompt-end",
        "modal-open",
        "ime-start",
        "route-close",
    ]:
        raise Cp51Error("UI invalidation policy changed")
    if ui.get("enter_behavior") != "dismiss-and-forward-native-submit":
        raise Cp51Error("Enter behavior changed")
    if ui.get("shortcut_policy") != "collision-checked-configurable-or-unbound":
        raise Cp51Error("shortcut policy changed")

    shells = document["shell_matrix"]
    if not isinstance(shells, list) or {row.get("id") for row in shells} != EXPECTED_SHELLS:
        raise Cp51Error("shell matrix changed")
    shell_keys = {"id", "owner", "adapter", "minimum", "shortcut", "verdict"}
    for row in shells:
        if set(row) != shell_keys:
            raise Cp51Error(f"shell {row.get('id')} keys changed")
    fish = next(row for row in shells if row["id"] == "fish")
    if fish["shortcut"] != "unbound-by-default-ctrl-space-collides":
        raise Cp51Error("Fish Ctrl+Space collision was lost")
    for shell_id in {"windows-powershell-5.1", "cmd"}:
        row = next(row for row in shells if row["id"] == shell_id)
        if row["verdict"] != "native-fallback-only":
            raise Cp51Error(f"{shell_id} must remain a truthful fallback")

    if document["limits"] != EXPECTED_LIMITS:
        raise Cp51Error("resource or latency limits changed")
    lifecycle = document["lifecycle"]
    if lifecycle.get("queue") != "one-latest-generation-per-pane":
        raise Cp51Error("queue policy changed")
    if lifecycle.get("worker") != "application-owned-joined-bounded":
        raise Cp51Error("worker ownership changed")
    if lifecycle.get("last_known_good") != "stale-marked-memory-only":
        raise Cp51Error("last-known-good policy changed")
    if lifecycle.get("kill_switch") != "runtime-immediate-no-restart-no-profile-change":
        raise Cp51Error("kill-switch policy changed")
    if lifecycle.get("disable_uninstall") != "remove-only-automexia-owned-session-artifacts":
        raise Cp51Error("disable/uninstall ownership changed")

    verification = document["verification"]
    required_domains = {
        "protocol",
        "security",
        "privacy",
        "ranking",
        "concurrency",
        "renderer",
        "accessibility",
        "shells",
        "performance",
        "resources",
        "rollback",
    }
    if set(verification) != required_domains or any(not verification[key] for key in verification):
        raise Cp51Error("verification ownership changed")
    if not isinstance(document["external_gates"], list) or len(document["external_gates"]) < 6:
        raise Cp51Error("external release gates are incomplete")

    canonical = json.dumps(
        document,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    if hashlib.sha256(canonical).hexdigest() != EXPECTED_CANONICAL_SHA256:
        raise Cp51Error("reviewed CP5.1 contract content changed")

    return {
        "threats": len(threats),
        "sources": len(sources),
        "shells": len(shells),
        "limits": len(EXPECTED_LIMITS),
    }


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    contract_text = _bounded_text(root / CONTRACT_PATH, MAX_CONTRACT_BYTES, "CP5.1 contract")
    counts = validate_contract(parse_contract(contract_text))
    adr = _bounded_text(root / ADR_PATH, MAX_DOCUMENT_BYTES, "CP5 editor-bridge ADR")
    plan = _bounded_text(root / PLAN_PATH, MAX_DOCUMENT_BYTES, "CP5 implementation audit")
    required_adr = [
        "Status: Proposed; CP5 runtime implementation and activation remain forbidden",
        "## Context",
        "## Proposed decision",
        "## Alternatives",
        "## Required acceptance and verification",
        "PIPE_REJECT_REMOTE_CLIENTS",
        "SO_PEERCRED",
        "getpeereid",
        "CP1",
    ]
    if any(fragment not in adr for fragment in required_adr):
        raise Cp51Error("CP5 editor-bridge ADR is missing a required boundary")
    required_plan = [
        "## Evidence ledger",
        "## CP5.1",
        "## CP5.2",
        "## CP5.3",
        "## CP5.4",
        "## CP5.5",
        "## CP5.6",
        "## Commit and rollback strategy",
        "External prerequisite",
    ]
    if any(fragment not in plan for fragment in required_plan):
        raise Cp51Error("CP5 implementation audit is incomplete")
    return counts


def main() -> int:
    try:
        counts = validate_repository()
    except (Cp51Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"CP5.1 proposal validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: CP5.1-CP5.6 proposal is non-activating and bounded "
        f"({counts['threats']} threats, {counts['sources']} sources, "
        f"{counts['shells']} shell modes, {counts['limits']} limits)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
