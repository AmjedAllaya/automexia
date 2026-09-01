#!/usr/bin/env python3
"""Validate the pinned Ghostty compatibility sources and assurance wiring."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
FIXTURE_ROOT = Path("tests/fixtures/keybindings/ghostty/1.3.1")
MAX_FILE_BYTES = 4 * 1024 * 1024
EXPECTED_VERSION = "1.3.1"
EXPECTED_COMMIT = "22efb0be2bbea73e5339f5426fa3b20edabcaa11"
EXPECTED_ARTIFACTS = {
    "actions.json",
    "automexia-classic-windows.json",
    "deviations.json",
    "linux.json",
    "provenance.json",
    "windows-adapted.json",
}


class GhosttyCompatibilityError(ValueError):
    """A compatibility fixture or mandatory assurance edge drifted."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise GhosttyCompatibilityError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def parse_json(text: str) -> Any:
    try:
        return json.loads(text, object_pairs_hook=_unique_object)
    except json.JSONDecodeError as error:
        raise GhosttyCompatibilityError(f"invalid JSON: {error}") from error


def _bounded_regular_bytes(path: Path) -> bytes:
    if path.is_symlink() or not path.is_file():
        raise GhosttyCompatibilityError(f"file is missing, linked, or irregular: {path}")
    size = path.stat().st_size
    if size <= 0 or size > MAX_FILE_BYTES:
        raise GhosttyCompatibilityError(
            f"file size must be within 1..{MAX_FILE_BYTES} bytes: {path}"
        )
    data = path.read_bytes()
    if len(data) != size:
        raise GhosttyCompatibilityError(f"file changed while reading: {path}")
    return data


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def validate_fixture_manifest(root: Path) -> dict[str, Any]:
    manifest = parse_json(_bounded_regular_bytes(root / "manifest.json").decode("utf-8"))
    if not isinstance(manifest, dict) or set(manifest) != {
        "schema_version",
        "version",
        "artifacts",
    }:
        raise GhosttyCompatibilityError("fixture manifest keys changed")
    if manifest["schema_version"] != 1 or manifest["version"] != EXPECTED_VERSION:
        raise GhosttyCompatibilityError("fixture manifest identity changed")
    artifacts = manifest["artifacts"]
    if not isinstance(artifacts, dict) or set(artifacts) != EXPECTED_ARTIFACTS:
        raise GhosttyCompatibilityError("fixture artifact inventory changed")
    for name, expected in artifacts.items():
        if not isinstance(expected, dict) or set(expected) != {"bytes", "sha256"}:
            raise GhosttyCompatibilityError(f"fixture metadata changed for {name}")
        data = _bounded_regular_bytes(root / name)
        if expected["bytes"] != len(data) or expected["sha256"] != _sha256(data):
            raise GhosttyCompatibilityError(f"fixture digest or size mismatch for {name}")

    provenance = parse_json(_bounded_regular_bytes(root / "provenance.json").decode("utf-8"))
    upstream = provenance.get("upstream", {}).get("upstream", {})
    if upstream.get("tag") != f"v{EXPECTED_VERSION}" or upstream.get("commit") != EXPECTED_COMMIT:
        raise GhosttyCompatibilityError("pinned upstream identity changed")
    if upstream.get("source_signature_verified") is not True:
        raise GhosttyCompatibilityError("upstream signature verification is not recorded")
    macos = provenance.get("macos_fixture")
    if macos != {
        "reason": "requires a native macOS Ghostty 1.3.1 generation and smoke runner",
        "status": "external_prerequisite",
    }:
        raise GhosttyCompatibilityError("macOS native fixture status became ambiguous")
    return manifest


def _read_source(relative: str) -> str:
    return _bounded_regular_bytes(ROOT / relative).decode("utf-8")


def repository_sources() -> dict[str, str]:
    return {
        "profile": _read_source("automexia-keybindings/src/profile.rs"),
        "xtask": _read_source("tools/xtask/src/main.rs"),
        "generator": _read_source("tools/xtask/src/keybindings.rs"),
        "nightly": _read_source(".github/workflows/nightly.yml"),
        "qa": _read_source("tools/ci/qa.py"),
        "roadmap": _read_source("docs/GHOSTTY-COMPATIBILITY-ROADMAP.md"),
        "adr": _read_source("docs/adr/0028-bounded-parked-pty-topology-history.md"),
        "context": _read_source("apps/automexia-terminal/src/context/mod.rs"),
        "inspector": _read_source(
            "apps/automexia-terminal/src/renderer/compatibility_inspector.rs"
        ),
        "screen": _read_source("apps/automexia-terminal/src/screen/mod.rs"),
        "screen_compatibility": _read_source(
            "apps/automexia-terminal/src/screen/compatibility.rs"
        ),
    }


def _require_tokens(source: str, tokens: tuple[str, ...], label: str) -> None:
    missing = [token for token in tokens if token not in source]
    if missing:
        raise GhosttyCompatibilityError(f"{label} is missing required tokens: {missing}")


def _require_before(source: str, first: str, second: str, label: str) -> None:
    first_index = source.find(first)
    second_index = source.find(second)
    if first_index < 0 or second_index < 0 or first_index >= second_index:
        raise GhosttyCompatibilityError(
            f"{label} must place {first!r} before {second!r}"
        )


def validate_gate_sources(sources: dict[str, str]) -> None:
    _require_tokens(
        sources["profile"],
        (
            'pub const GHOSTTY_1_3_PATCH: &str = "1.3.1";',
            f'pub const GHOSTTY_1_3_COMMIT: &str = "{EXPECTED_COMMIT}";',
            "Self::Automexia",
            "requires the checked-in native macOS fixture",
        ),
        "profile owner",
    )
    _require_tokens(
        sources["xtask"],
        ("keybindings::verify()?;", "run_python(\"tools/ci/check_ghostty_compatibility.py\")?;"),
        "verify-all gate",
    )
    _require_tokens(
        sources["generator"],
        ("generated_outputs_with_classic", 'outputs.insert("automexia-classic-windows.json"'),
        "fixture generator",
    )
    _require_tokens(
        sources["nightly"],
        (
            "ecosystem_bundle",
            "ghostty_keybindings",
            "ghostty_migration",
            "-p automexia-keybindings --no-run --locked",
        ),
        "nightly compatibility jobs",
    )
    _require_tokens(
        sources["qa"],
        (
            '"ghostty-assurance-mutations"',
            '"benchmark-keybindings",',
            '["cargo", "bench", "-p", "automexia-keybindings", "--bench", "registry"',
            'os.environ.get("AUTOMEXIA_QA_GHOSTTY_EVIDENCE", "").strip()',
            '"tools/ci/ghostty_native_evidence.py"',
        ),
        "QA compatibility evidence",
    )
    _require_tokens(
        sources["roadmap"],
        ("G0 — source lock", "G5 — tooling", "G6 — high-lifecycle"),
        "canonical roadmap",
    )
    _require_tokens(
        sources["adr"],
        ("Status: accepted", "memory-only", "bounded", "two-step confirmation"),
        "lifecycle ADR",
    )
    _require_tokens(
        sources["context"],
        (
            "pub struct ParkedTopologySummary",
            "pub fn parked_topology_summaries",
            "pub fn clear_parked_topologies",
            "parked_topology_summary_is_redacted_bounded_and_clear_is_exact",
        ),
        "parked topology owner",
    )
    _require_tokens(
        sources["inspector"],
        (
            "pub struct ParkedTopologyPresentation",
            "ConfirmClearParked",
            "request_clear_confirmation",
            "accessibility_summary",
            "parked_controls_are_bounded_distinct_and_pointer_accessible",
        ),
        "redacted compatibility inspector",
    )
    _require_tokens(
        sources["screen"],
        (
            "if self.process_compatibility_inspector_key(key)",
            "CompatibilityInspectorAction::ConfirmClearParked",
            'snapshot["compatibility_inspector_accessibility_summary"]',
            "compatibility_inspector_key_policy_is_modal_idempotent_and_confirmed",
        ),
        "screen modal boundary",
    )
    _require_before(
        sources["screen"],
        "if self.process_compatibility_inspector_key(key)",
        "if self.handle_image_preview_key(key)",
        "modal input precedence",
    )
    _require_tokens(
        sources["screen_compatibility"],
        (
            "parked_topology_summaries()",
            "process_compatibility_inspector_key",
            "clear_parked_topologies()",
            "undo_topology(&mut self.sugarloaf)",
        ),
        "screen lifecycle adapter",
    )


def validate_repository() -> dict[str, Any]:
    manifest = validate_fixture_manifest(ROOT / FIXTURE_ROOT)
    validate_gate_sources(repository_sources())
    return {
        "version": EXPECTED_VERSION,
        "fixture_artifacts": len(manifest["artifacts"]),
        "nightly_fuzz_targets": 2,
        "external_native_platforms": 3,
    }


def main() -> int:
    try:
        result = validate_repository()
    except (GhosttyCompatibilityError, UnicodeDecodeError, OSError) as error:
        print(f"FAIL: Ghostty compatibility assurance: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: pinned Ghostty compatibility fixtures and local assurance wiring "
        f"({result['fixture_artifacts']} artifacts, {result['nightly_fuzz_targets']} fuzzers)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
