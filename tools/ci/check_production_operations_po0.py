#!/usr/bin/env python3
"""Validate the public, non-activating Production Operations boundary.

The exact unreleased schemas, ranking rules, provider mappings, and delivery
ledger are intentionally local-only. This checker protects the public promise:
the compatibility path reports no implementation, preserves its trust
boundaries, exposes no private planning contract, and gains no runtime
authority by accident.
"""

from __future__ import annotations

from pathlib import Path
import re
import sys

from check_repository_aligned_docs import PUBLIC_FUTURE_PLANNING


ROOT = Path(__file__).resolve().parents[2]
MAX_DOCUMENT_BYTES = 2 * 1024 * 1024
PRIVATE_FIXTURE = Path("tests/fixtures/production-operations/po0-contract-v1.json")
REQUIRED_DOCUMENTS = {
    Path("docs/SITUATION-AWARE-PRODUCTION-OPERATIONS.md"): (
        "Status: no public runtime implementation or activation.",
        "## Architectural boundary",
        "## Security and privacy threat model",
        "Detailed ranking formulas, provider playbooks, schemas, internal limits, and",
    ),
    Path("docs/SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md"): (
        "Status: no public runtime implementation or activation.",
        "a suggestion cannot execute itself, type into a PTY, or imply approval",
        "Detailed schemas, scoring fields, provider mappings, and wire formats remain",
    ),
    Path("docs/SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md"): (
        "Status: no public runtime implementation or activation.",
        "## Interaction requirements",
    ),
    Path("docs/SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md"): (
        "Status: no public runtime implementation or activation.",
        "## Scenario inventory",
        "## Phase exit criteria",
    ),
    Path("docs/adr/0034-situation-aware-production-operations.md"): (
        "Status: no public runtime implementation or activation.",
        "## Proposed decision",
        "Detailed internal contracts and execution recipes remain local",
    ),
}
FORBIDDEN_PUBLIC_MARKERS = (
    "8038ce24aa0f4223a910ed2293f9c40b17882e617fda9a01a14d13f65d7004cf",
    "### Exact payload catalog",
    "### Stable lexicographic order",
    '"phase": "PO0"',
    '"record_payloads"',
    "PO-R01",
)
FORBIDDEN_RUNTIME_PATHS = (
    Path("automexia-operations-model"),
    Path("automexia-devops/src/operations"),
    Path("apps/automexia-terminal/src/automexia/operations"),
    Path("apps/automexia-terminal/src/screen/operations.rs"),
    Path("apps/automexia-terminal/src/renderer/operations.rs"),
)
FORBIDDEN_SOURCE_MARKERS = (
    "RefreshProductionContext",
    "OpenProductionSituation",
    "ReviewOperationalCandidate",
    "ToggleIncidentMode",
    "StopManagedOperation",
    "production_operations",
    "ProductionOperationsConfig",
)
SOURCE_ROOTS = (
    Path("apps/automexia-terminal/src"),
    Path("automexia-devops/src"),
    Path("automexia-ui-model/src"),
    Path("rio-backend/src"),
)


class ProductionOperationsBoundaryError(ValueError):
    """The public planning boundary leaked detail or gained runtime authority."""


def _bounded_text(path: Path, label: str) -> str:
    if path.is_symlink() or not path.is_file():
        raise ProductionOperationsBoundaryError(
            f"{label} must be a regular, non-linked file"
        )
    payload = path.read_bytes()
    if not payload or len(payload) > MAX_DOCUMENT_BYTES:
        raise ProductionOperationsBoundaryError(
            f"{label} must contain 1..{MAX_DOCUMENT_BYTES} bytes"
        )
    try:
        return payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ProductionOperationsBoundaryError(
            f"{label} must be strict UTF-8"
        ) from error


def _validate_public_documents(root: Path) -> int:
    for relative, markers in REQUIRED_DOCUMENTS.items():
        text = _bounded_text(root / relative, relative.as_posix())
        missing = [marker for marker in markers if marker not in text]
        if missing:
            raise ProductionOperationsBoundaryError(
                f"{relative.as_posix()} lost public boundary markers"
            )
        leaked = [marker for marker in FORBIDDEN_PUBLIC_MARKERS if marker in text]
        if leaked:
            raise ProductionOperationsBoundaryError(
                f"{relative.as_posix()} exposes local-only planning detail"
            )
        if PUBLIC_FUTURE_PLANNING.search(text):
            raise ProductionOperationsBoundaryError(
                f"{relative.as_posix()} exposes future planning"
            )

    fixture = root / PRIVATE_FIXTURE
    if fixture.exists() or fixture.is_symlink():
        raise ProductionOperationsBoundaryError(
            f"local-only planning fixture must not be tracked: {PRIVATE_FIXTURE.as_posix()}"
        )
    return len(REQUIRED_DOCUMENTS)


def _validate_nonactivation(root: Path) -> int:
    for relative in FORBIDDEN_RUNTIME_PATHS:
        path = root / relative
        if path.exists() or path.is_symlink():
            raise ProductionOperationsBoundaryError(
                f"planned feature unexpectedly gained runtime path {relative.as_posix()}"
            )

    checked = 0
    for relative_root in SOURCE_ROOTS:
        source_root = root / relative_root
        if not source_root.exists():
            continue
        for path in source_root.rglob("*.rs"):
            checked += 1
            text = _bounded_text(path, path.relative_to(root).as_posix())
            marker = next(
                (item for item in FORBIDDEN_SOURCE_MARKERS if item in text),
                None,
            )
            if marker:
                raise ProductionOperationsBoundaryError(
                    f"{path.relative_to(root).as_posix()} gained activating marker {marker!r}"
                )

    cargo = _bounded_text(root / "Cargo.toml", "Cargo.toml")
    if re.search(r"automexia[-_]operations[-_]model", cargo):
        raise ProductionOperationsBoundaryError(
            "workspace unexpectedly gained a Production Operations model crate"
        )
    for relative in (
        Path("docs/CONFIGURATION.md"),
        Path("docs/reference/configuration.md"),
    ):
        text = _bounded_text(root / relative, relative.as_posix())
        if "[production-operations]" in text:
            raise ProductionOperationsBoundaryError(
                f"{relative.as_posix()} claims unimplemented settings"
            )
    return checked


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    return {
        "documents": _validate_public_documents(root),
        "source_files_checked": _validate_nonactivation(root),
        "private_artifacts": 0,
    }


def main() -> int:
    try:
        counts = validate_repository()
    except (OSError, UnicodeError, ProductionOperationsBoundaryError) as error:
        print(f"Production Operations boundary validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: Production Operations remains public-summary-only and non-activating "
        f"(documents={counts['documents']}, source-files={counts['source_files_checked']}, "
        f"private-artifacts={counts['private_artifacts']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
