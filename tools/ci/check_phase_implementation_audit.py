#!/usr/bin/env python3
"""Validate that the cross-roadmap phase audit stays complete and explicit."""

from __future__ import annotations

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
AUDIT_PATH = "docs/PHASE-IMPLEMENTATION-AUDIT.md"
CANONICAL_PHASE_SOURCES = (
    "docs/ROADMAP.md",
    "docs/STABILIZATION-ROADMAP.md",
    "docs/SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md",
    "docs/CONNECTION-HUB.md",
    "docs/COMMAND-PRODUCTIVITY.md",
    "docs/DEVOPS-ALIASES.md",
    "docs/GHOSTTY-COMPATIBILITY-ROADMAP.md",
)
REQUIRED_GLOBAL_HEADINGS = (
    "# Phase implementation audit",
    "## Status rules",
    "## Audit method and current evidence",
    "## Executive phase matrix",
    "### Architecture Phase 0-5 mapping",
    "## Core stabilization phases",
    "## DevOps, SSH, and multi-cloud phases",
    "## Command-productivity phases",
    "## Ghostty compatibility phases",
    "## Version milestone assessment",
    "## Cross-cutting quality assessment",
    "## Platform assurance",
    "## Test and benchmark tiers",
    "## Retry, recovery, and failure policy",
    "## Recommended execution order",
    "## Final assessment",
)
REQUIRED_PHASE_PREFIXES = (
    "### S0.1 ",
    "### S0.2 ",
    "### S0.3 ",
    "### S1.1 ",
    "### S1.2 ",
    "### S1.3 ",
    "### S1.4 ",
    "### S1.5 ",
    "### S1.6 ",
    "### S1.7 ",
    "### S1.8 and S2 ",
    "### v0.5 assurance maturation",
    "### D0 ",
    "### D1 ",
    "### D2 ",
    "### D3 ",
    "### D4 ",
    "### D5.0 ",
    "### D5.1 ",
    "### D5.2 ",
    "### D6.0-D6.5 ",
    "### D7 ",
    "### CP0 ",
    "### CP1 ",
    "### CP2.0 ",
    "### CP2.1 ",
    "### CP2.2 ",
    "### CP3.0 ",
    "### CP3.1 ",
    "### CP3.2 ",
    "### CP3.3 ",
    "### CP4 ",
    "### CP5.0-CP5.6 ",
    "### CP6 ",
    "### G0 ",
    "### G1 ",
    "### G2 ",
    "### G3 ",
    "### G4 ",
    "### G5 ",
    "### G6 ",
    "### v0.4 stable",
    "### v0.5.0 foundation and SSH",
    "### v0.5.1 multi-cloud",
    "### v0.6 ecosystem and AI",
)
REQUIRED_EVIDENCE_TERMS = (
    "correctness",
    "security",
    "performance",
    "resource",
    "storage",
    "resilience",
    "retry",
    "recovery",
    "architecture",
    "code quality",
    "documentation",
    "accessibility",
    "visual quality",
    "Windows",
    "Linux",
    "macOS",
    "test",
    "benchmark",
    "fuzz",
    "coverage",
    "release",
)
PHASE_ID = re.compile(
    r"\b(?:S\d(?:\.\d+)?|D\d(?:\.\d+)?|CP\d(?:\.\d+)?|"
    r"G\d(?:\.\d+)?|Phase [0-9]+)\b"
)
SUBPHASE_RANGE = re.compile(
    r"\b(?P<prefix>S|D|CP|G)(?P<major>\d+)\.(?P<start>\d+)-"
    r"(?P=prefix)(?P=major)\.(?P<end>\d+)\b"
)
NUMBERED_PHASE_RANGE = re.compile(r"\bPhase (?P<start>\d+)-(?P<end>\d+)\b")
STATUS = re.compile(
    r"\*\*[^\n]*(?:fully implemented|partially implemented|not implemented|"
    r"partial|blocked|deferred)[^\n]*\*\*",
    re.IGNORECASE,
)


class PhaseAuditError(ValueError):
    """The implementation audit no longer covers its roadmap contract."""


def unique_heading_index(lines: list[str], prefix: str) -> int:
    matches = [index for index, line in enumerate(lines) if line.startswith(prefix)]
    if len(matches) != 1:
        raise PhaseAuditError(
            f"phase audit requires exactly one heading beginning {prefix!r}; "
            f"found {len(matches)}"
        )
    return matches[0]


def section_body(lines: list[str], heading_index: int) -> str:
    body: list[str] = []
    for line in lines[heading_index + 1 :]:
        if line.startswith("## ") or line.startswith("### "):
            break
        body.append(line)
    return "\n".join(body)


def phase_is_covered(required: str, audit_ids: set[str]) -> bool:
    if required in audit_ids:
        return True
    if re.fullmatch(r"[SDG]\d|CP\d", required):
        return any(candidate.startswith(required + ".") for candidate in audit_ids)
    return False


def audit_heading_phase_ids(lines: list[str]) -> set[str]:
    """Return phase IDs owned by audit headings, expanding declared ranges."""
    ids: set[str] = set()
    for line in lines:
        if not re.match(r"^#{2,4}\s+", line):
            continue
        ids.update(PHASE_ID.findall(line))
        for match in SUBPHASE_RANGE.finditer(line):
            start = int(match.group("start"))
            end = int(match.group("end"))
            if end < start:
                raise PhaseAuditError(f"descending phase range in heading {line!r}")
            ids.update(
                f"{match.group('prefix')}{match.group('major')}.{minor}"
                for minor in range(start, end + 1)
            )
        for match in NUMBERED_PHASE_RANGE.finditer(line):
            start = int(match.group("start"))
            end = int(match.group("end"))
            if end < start:
                raise PhaseAuditError(f"descending phase range in heading {line!r}")
            ids.update(f"Phase {phase}" for phase in range(start, end + 1))
    return ids


def validate_text(
    audit: str,
    canonical_phase_ids: set[str],
    canonical_sources: tuple[str, ...] = CANONICAL_PHASE_SOURCES,
) -> dict[str, int]:
    lines = audit.splitlines()
    if not lines or lines[0] != "# Phase implementation audit":
        raise PhaseAuditError("phase audit must begin with one canonical title")

    for heading in REQUIRED_GLOBAL_HEADINGS:
        if lines.count(heading) != 1:
            raise PhaseAuditError(
                f"phase audit requires exactly one global heading {heading!r}"
            )

    for source in canonical_sources:
        label = Path(source).name
        if f"({label})" not in audit:
            raise PhaseAuditError(f"phase audit does not link canonical source {source}")

    for prefix in REQUIRED_PHASE_PREFIXES:
        index = unique_heading_index(lines, prefix)
        if not STATUS.search(section_body(lines, index)):
            raise PhaseAuditError(
                f"phase section {lines[index]!r} has no explicit bold status"
            )

    folded = audit.casefold()
    missing_terms = sorted(
        term for term in REQUIRED_EVIDENCE_TERMS if term.casefold() not in folded
    )
    if missing_terms:
        raise PhaseAuditError(
            f"phase audit is missing cross-cutting evidence terms: {missing_terms}"
        )

    audit_ids = audit_heading_phase_ids(lines)
    uncovered = sorted(
        phase
        for phase in canonical_phase_ids
        if not phase_is_covered(phase, audit_ids)
    )
    if uncovered:
        raise PhaseAuditError(
            f"phase audit does not cover canonical roadmap phases: {uncovered}"
        )

    baseline = re.search(r"Audited source baseline: ([0-9a-f]{40})", audit)
    if baseline is None:
        raise PhaseAuditError("phase audit must record one 40-character source baseline")

    return {
        "phase_sections": len(REQUIRED_PHASE_PREFIXES),
        "canonical_phases": len(canonical_phase_ids),
        "evidence_dimensions": len(REQUIRED_EVIDENCE_TERMS),
        "source_documents": len(canonical_sources),
    }


def canonical_phase_ids(root: Path = ROOT) -> set[str]:
    phases: set[str] = set()
    for relative in CANONICAL_PHASE_SOURCES:
        path = root / relative
        if not path.is_file():
            raise PhaseAuditError(f"missing canonical roadmap source {relative}")
        for line in path.read_text(encoding="utf-8").splitlines():
            if not re.match(r"^#{2,4}\s+", line):
                continue
            phases.update(PHASE_ID.findall(line))
    return phases


def validate(root: Path = ROOT) -> dict[str, int]:
    path = root / AUDIT_PATH
    if not path.is_file():
        raise PhaseAuditError(f"missing phase implementation audit {AUDIT_PATH}")
    return validate_text(
        path.read_text(encoding="utf-8"),
        canonical_phase_ids(root),
    )


if __name__ == "__main__":
    try:
        counts = validate()
    except Exception as error:  # noqa: BLE001 - print the exact contract failure.
        print(f"phase implementation audit validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
    print(
        "PASS: phase implementation audit is complete "
        f"(sections={counts['phase_sections']}, "
        f"canonical_phases={counts['canonical_phases']}, "
        f"evidence_dimensions={counts['evidence_dimensions']}, "
        f"sources={counts['source_documents']})"
    )
