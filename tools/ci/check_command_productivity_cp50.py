#!/usr/bin/env python3
"""Validate the bounded, non-activating CP5.0 autocomplete research contract."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = Path(
    "tests/fixtures/command-productivity/cp50-research-contract-v1.json"
)
MAX_CONTRACT_BYTES = 65_536
MAX_SOURCE_BYTES = 1_048_576
EXPECTED_TOP_LEVEL_KEYS = {
    "schema",
    "phase",
    "status",
    "decision",
    "capabilities",
    "limits",
    "shell_matrix",
    "bridge_prototype",
    "benchmark",
    "dependency_decisions",
    "evidence_dimensions",
    "external_gates",
}
EXPECTED_DECISION = {
    "p2": "deferred",
    "default_experience": "cp1-shell-native",
    "runtime_dependency": "none",
    "runtime_surface": "none",
    "review_required_to_change": True,
    "rationale": (
        "No single supported, mutation-free, persistent editor-state bridge "
        "preserves every native shell owner; CP1 already supplies the complete "
        "reversible completion path."
    ),
}
EXPECTED_CAPABILITIES = {
    "runtime_activation": False,
    "process_spawn": False,
    "pty_access": False,
    "network": False,
    "secret_read": False,
    "history_read": False,
    "buffer_persistence": False,
    "terminal_grid_inference": False,
    "profile_mutation": False,
    "keybinding_mutation": False,
    "implicit_enter": False,
}
EXPECTED_LIMITS = {
    "buffer_bytes": 16_384,
    "candidate_count": 512,
    "candidate_bytes": 1_024,
    "batch_bytes": 524_288,
    "matcher_deadline_ms": 16,
    "benchmark_warmups": 20,
    "benchmark_samples": 200,
}
EXPECTED_SHELLS = {
    "powershell-7",
    "windows-powershell-5.1",
    "bash",
    "zsh",
    "fish",
    "cmd",
    "wsl",
}
EXPECTED_CORPUS_SIZES = [32, 128, 512]
EXPECTED_CASES = [
    "ascii-prefix-and-token",
    "unicode-and-combining",
    "long-common-prefix",
    "stale-generation-cancellation",
]
EXPECTED_DEPENDENCIES = {
    "in-tree-deterministic-matcher": "retain",
    "nucleo-matcher": "research-only-rejected-for-runtime",
    "reedline": "reference-only",
    "carapace": "explicit-external-adapter-only",
}
EXPECTED_EVIDENCE = {
    "native_completion_and_prediction",
    "startup",
    "typing_latency",
    "memory",
    "cancellation",
    "resize",
    "accessibility",
    "disable",
    "privacy",
}


class Cp50Error(ValueError):
    """The CP5.0 research boundary is invalid."""


@dataclass(frozen=True)
class EditorSnapshot:
    """Ephemeral research model for state already owned by a native editor."""

    buffer: str
    cursor_byte: int
    replacement_start_byte: int
    replacement_end_byte: int
    generation: int


@dataclass(frozen=True)
class CandidateBatch:
    """Bounded generation-tagged candidate result returned to a native editor."""

    generation: int
    candidates: tuple[str, ...]


@dataclass(frozen=True)
class InsertionRequest:
    """Replacement-only request; execution remains impossible by construction."""

    start_byte: int
    end_byte: int
    replacement: str
    generation: int
    execute: bool = False


def _bounded_text(path: Path, maximum: int, label: str) -> str:
    if path.is_symlink():
        raise Cp50Error(f"{label} must not be a symbolic link: {path}")
    size = path.stat().st_size
    if size > maximum:
        raise Cp50Error(f"{label} exceeds {maximum} bytes: {path}")
    return path.read_text(encoding="utf-8")
def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    document: dict[str, Any] = {}
    for key, value in pairs:
        if key in document:
            raise Cp50Error(f"CP5.0 contract contains duplicate key {key!r}")
        document[key] = value
    return document


def parse_contract(text: str) -> Any:
    return json.loads(text, object_pairs_hook=_unique_object)


def _utf8_boundaries(value: str) -> set[int]:
    boundaries = {0}
    length = 0
    for character in value:
        length += len(character.encode("utf-8"))
        boundaries.add(length)
    return boundaries


def _is_unsafe_format_character(character: str) -> bool:
    codepoint = ord(character)
    return (
        0x0080 <= codepoint <= 0x009F
        or 0x200B <= codepoint <= 0x200F
        or 0x202A <= codepoint <= 0x202E
        or 0x2060 <= codepoint <= 0x2064
        or 0x2066 <= codepoint <= 0x2069
        or codepoint in {0x061C, 0xFEFF}
    )


def build_insertion_request(
    snapshot: EditorSnapshot,
    batch: CandidateBatch,
    selected_index: int,
) -> InsertionRequest:
    """Validate untrusted prototype state and return a shell-owned replacement."""

    integer_fields = (
        snapshot.cursor_byte,
        snapshot.replacement_start_byte,
        snapshot.replacement_end_byte,
        snapshot.generation,
        batch.generation,
        selected_index,
    )
    if any(type(value) is not int for value in integer_fields):
        raise Cp50Error("editor offsets, generations, and selection must be integers")
    if (
        not isinstance(snapshot.buffer, str)
        or not isinstance(batch.candidates, tuple)
    ):
        raise Cp50Error("editor buffer and candidate batch have invalid types")
    buffer_size = len(snapshot.buffer.encode("utf-8"))

    if buffer_size > EXPECTED_LIMITS["buffer_bytes"]:
        raise Cp50Error("editor buffer exceeds the research ceiling")
    if snapshot.generation < 0 or batch.generation != snapshot.generation:
        raise Cp50Error("stale generation must be rejected before insertion")
    if len(batch.candidates) > EXPECTED_LIMITS["candidate_count"]:
        raise Cp50Error("candidate count exceeds the research ceiling")
    if not 0 <= selected_index < len(batch.candidates):
        raise Cp50Error("selected candidate index is outside the batch")
    boundaries = _utf8_boundaries(snapshot.buffer)
    span = {
        snapshot.cursor_byte,
        snapshot.replacement_start_byte,
        snapshot.replacement_end_byte,
    }
    if not span.issubset(boundaries):
        raise Cp50Error("cursor and replacement span must be on a UTF-8 boundary")
    if not (
        0
        <= snapshot.replacement_start_byte
        <= snapshot.cursor_byte
        <= snapshot.replacement_end_byte
        <= buffer_size
    ):
        raise Cp50Error("replacement span is invalid")

    batch_bytes = 0
    for candidate in batch.candidates:
        if not isinstance(candidate, str):
            raise Cp50Error("candidate batch contains a non-text value")
        candidate_bytes = candidate.encode("utf-8")
        if not candidate or len(candidate_bytes) > EXPECTED_LIMITS["candidate_bytes"]:
            raise Cp50Error("candidate exceeds the replacement ceiling")
        if any(ord(character) < 0x20 or ord(character) == 0x7F for character in candidate):
            raise Cp50Error("candidate contains a control character")
        if any(_is_unsafe_format_character(character) for character in candidate):
            raise Cp50Error("candidate contains an unsafe character")
        batch_bytes += len(candidate_bytes)
        if batch_bytes > EXPECTED_LIMITS["batch_bytes"]:
            raise Cp50Error("candidate batch exceeds the byte ceiling")

    replacement = batch.candidates[selected_index]
    return InsertionRequest(
        start_byte=snapshot.replacement_start_byte,
        end_byte=snapshot.replacement_end_byte,
        replacement=replacement,
        generation=snapshot.generation,
    )


def validate_contract(document: Any) -> dict[str, int]:
    if not isinstance(document, dict) or set(document) != EXPECTED_TOP_LEVEL_KEYS:
        raise Cp50Error("CP5.0 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1,
        "CP5.0",
        "research-complete-cp1-retained",
    ):
        raise Cp50Error("CP5.0 status changed")
    if document["decision"] != EXPECTED_DECISION:
        raise Cp50Error("CP5.0 decision changed without review")
    if document["capabilities"] != EXPECTED_CAPABILITIES:
        raise Cp50Error("CP5.0 capability boundary changed")
    if document["limits"] != EXPECTED_LIMITS:
        raise Cp50Error("CP5.0 resource ceilings changed")

    rows = document["shell_matrix"]
    if not isinstance(rows, list) or len(rows) != len(EXPECTED_SHELLS):
        raise Cp50Error("CP5.0 shell matrix changed")
    ids = {row.get("id") for row in rows if isinstance(row, dict)}
    required_row_keys = {
        "id",
        "baseline",
        "native_owner",
        "prototype",
        "verdict",
        "local_evidence",
    }
    if ids != EXPECTED_SHELLS or any(set(row) != required_row_keys for row in rows):
        raise Cp50Error("CP5.0 shell matrix changed")
    if any(row["verdict"] != "native-only" for row in rows):
        raise Cp50Error("CP5.0 shell matrix must retain native editor ownership")

    bridge = document["bridge_prototype"]
    if bridge != {
        "owner": "research-only pure model",
        "source": "tools/ci/check_command_productivity_cp50.py",
        "state": [
            "ephemeral buffer",
            "UTF-8 byte cursor",
            "shell-owned replacement span",
            "generation",
        ],
        "candidate_result": ["generation", "bounded replacement text"],
        "insertion_owner": "native-shell-editor",
        "transport": "none",
        "persistence": "none",
        "execution": "never",
    }:
        raise Cp50Error("CP5.0 bridge prototype changed")

    benchmark = document["benchmark"]
    if (
        benchmark.get("manifest")
        != "tools/research/cp5-matcher-benchmark/Cargo.toml"
        or benchmark.get("report") != "docs/research/CP5-AUTOCOMPLETE-RESEARCH.md"
        or benchmark.get("candidate") != "nucleo-matcher=0.3.1"
        or benchmark.get("corpus_sizes") != EXPECTED_CORPUS_SIZES
        or benchmark.get("cases") != EXPECTED_CASES
        or benchmark.get("publish_rule")
        != "reject-stale-generation-before-publish"
        or benchmark.get("runtime_dependency") is not False
        or set(benchmark)
        != {
            "manifest",
            "report",
            "candidate",
            "corpus_sizes",
            "cases",
            "publish_rule",
            "runtime_dependency",
        }
    ):
        raise Cp50Error("CP5.0 benchmark contract changed")

    dependencies = document["dependency_decisions"]
    actual_dependencies = {
        row.get("name"): row.get("role")
        for row in dependencies
        if isinstance(row, dict) and set(row) == {"name", "role", "boundary"}
    }
    if (
        actual_dependencies != EXPECTED_DEPENDENCIES
        or len(dependencies) != len(EXPECTED_DEPENDENCIES)
    ):
        raise Cp50Error("CP5.0 dependency decision changed")
    if set(document["evidence_dimensions"]) != EXPECTED_EVIDENCE:
        raise Cp50Error("CP5.0 evidence dimensions changed")
    if not isinstance(document["external_gates"], list) or len(document["external_gates"]) != 4:
        raise Cp50Error("CP5.0 external gates changed")
    return {
        "shells": len(rows),
        "corpus_sizes": len(EXPECTED_CORPUS_SIZES),
        "dependency_decisions": len(dependencies),
    }


def validate_sources(root: Path = ROOT) -> dict[str, int]:
    required_files = {
        "tools/research/cp5-matcher-benchmark/Cargo.toml": {
            'publish = false',
            'nucleo-matcher = { version = "=0.3.1"',
            "[workspace]",
        },
        "tools/research/cp5-matcher-benchmark/src/main.rs": {
            "CORPUS_SIZES",
            "WARMUPS",
            "SAMPLES",
            "stale_generation",
            "unicode",
            "Pattern::new",
        },
        "docs/research/CP5-AUTOCOMPLETE-RESEARCH.md": {
            "# CP5.0 native autocomplete research",
            "## Evidence ledger",
            "## Shell and version decision matrix",
            "## Matcher comparison",
            "## Build, wrap, or adopt decision",
            "## Privacy and authority delta",
            "## External evidence",
            "CP1 remains the complete solution",
        },
        "docs/COMMAND-PRODUCTIVITY.md": {
            "CP5.0 research decision",
            "cp50-research-contract-v1.json",
        },
        "docs/TESTING.md": {
            "Command-productivity CP5.0 research",
            "test_command_productivity_cp50.py",
        },
        "docs/CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md": {
            "P1 - execute CP5.0 native autocomplete research",
            "Status: Fully done",
        },
        "docs/ROADMAP.md": {
            "**Fully done** | CP5.0",
            "**Partially done** | CP5.1-CP5.6",
        },
        "docs/PHASE-IMPLEMENTATION-AUDIT.md": {
            "CP5.0 — autocomplete research",
            "**Fully implemented at the research boundary.**",
        },
        "docs/STABILIZATION-ROADMAP.md": {
            "CP5.0 research — **Fully done**",
            "CP5.1 bridge",
        },
        "docs/COMMAND-PRODUCTIVITY-COMPATIBILITY.md": {
            "CP5.0 fully done",
            "CP5.1-CP5.3 are fully implemented at source/local model boundaries",
        },
        "docs/project/roadmap.md": {
            "CP5.0",
            "**Fully implemented at research boundary**",
        },
        "docs/index.md": {
            "Accepted source work partial overall",
            "research/CP5-AUTOCOMPLETE-RESEARCH.md",
        },
        "tools/ci/validate_repository.py": {"validate_command_productivity_cp50"},
        "tools/ci/qa.py": {
            "cp50-research-clippy",
            "cp50-research-benchmark",
        },
        ".github/workflows/ci.yml": {
            "test_command_productivity_cp50.py",
            "CP5.0 standalone matcher research gates",
        },
        "tests/assurance/feature-matrix.json": {
            "command-productivity-cp50-research",
            "CP5-AUTOCOMPLETE-RESEARCH.md",
        },
        "changes/0.5.0-cp50-autocomplete-research.md": {
            "# CP5.0 native autocomplete research",
            "No user configuration or migration is required",
        },
    }
    for relative, tokens in required_files.items():
        path = root / relative
        if not path.is_file():
            raise Cp50Error(f"CP5.0 evidence file is missing: {relative}")
        text = _bounded_text(path, MAX_SOURCE_BYTES, "CP5.0 evidence")
        missing = sorted(token for token in tokens if token not in text)
        if missing:
            raise Cp50Error(f"{relative} is missing CP5.0 evidence: {missing}")

    root_manifest = _bounded_text(
        root / "Cargo.toml", MAX_SOURCE_BYTES, "workspace manifest"
    )
    root_lock = _bounded_text(
        root / "Cargo.lock", 16 * MAX_SOURCE_BYTES, "workspace lock"
    )
    if (
        "nucleo-matcher" in root_manifest
        or 'name = "nucleo-matcher"' in root_lock
    ):
        raise Cp50Error("nucleo-matcher must not enter the runtime workspace")
    return {"evidence_files": len(required_files)}


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    contract_path = root / CONTRACT_PATH
    contract = parse_contract(
        _bounded_text(contract_path, MAX_CONTRACT_BYTES, "CP5.0 contract")
    )
    counts = validate_contract(contract)
    counts.update(validate_sources(root))
    return counts


def main() -> int:
    try:
        counts = validate_repository()
    except (Cp50Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"command productivity CP5.0 validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: CP5.0 research is bounded and non-activating "
        f"(shells={counts['shells']}, corpus_sizes={counts['corpus_sizes']}, "
        f"dependency_decisions={counts['dependency_decisions']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
