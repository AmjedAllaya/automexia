#!/usr/bin/env python3
"""Validate feature-to-quality and native-platform assurance traceability."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
import tomllib
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MATRIX = ROOT / "tests/assurance/feature-matrix.json"
QUALITY_DIMENSIONS = {
    "accessibility",
    "correctness",
    "security",
    "performance",
    "resource_lifetime",
    "storage_hygiene",
    "resilience",
    "visual_quality",
}
SUPPORTED_PLATFORMS = {"windows", "linux", "macos"}
LEVELS = {"pr", "nightly", "controlled", "external", "not_applicable"}
DOCUMENTATION_TYPES = {"guide", "reference", "explanation"}
REQUIRED_SURFACES = {
    ".github/workflows",
    "packaging",
    "shell-integration",
    "tools/xtask",
}
EXCLUDED_TREE_PARTS = {".git", ".automexia-private", "target"}


class AssuranceError(ValueError):
    """The assurance matrix violates its contract."""


def benchmark_targets(root: Path) -> set[str]:
    """Return product benchmark owners without scanning generated/private copies."""
    return {
        path.relative_to(root).as_posix()
        for path in root.glob("**/benches/*.rs")
        if not EXCLUDED_TREE_PARTS.intersection(path.relative_to(root).parts)
    }


def workspace_members(root: Path) -> set[str]:
    with (root / "Cargo.toml").open("rb") as source:
        manifest = tomllib.load(source)
    return {Path(member).as_posix().rstrip("/") for member in manifest["workspace"]["members"]}


def evidence_path(root: Path, reference: str) -> Path:
    path_text = reference.split("#", 1)[0].split("::", 1)[0]
    path = root / Path(path_text)
    try:
        path.resolve().relative_to(root.resolve())
    except ValueError as error:
        raise AssuranceError(f"evidence escapes the repository: {reference}") from error
    return path


def markdown_anchors(path: Path) -> set[str]:
    anchors: set[str] = set()
    occurrences: dict[str, int] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = re.match(r"^#{1,6}\s+(.+?)\s*#*\s*$", line)
        if not match:
            continue
        heading = re.sub(r"<[^>]+>", "", match.group(1)).strip().lower()
        slug = re.sub(r"[^\w\- ]", "", heading, flags=re.UNICODE)
        slug = re.sub(r"[\s-]+", "-", slug).strip("-")
        duplicate = occurrences.get(slug, 0)
        occurrences[slug] = duplicate + 1
        anchors.add(slug if duplicate == 0 else f"{slug}-{duplicate}")
    return anchors


def validate_fragment(path: Path, reference: str) -> None:
    _, separator, fragment = reference.partition("#")
    if not separator or not fragment:
        return
    if path.suffix.lower() == ".md" and fragment not in markdown_anchors(path):
        raise AssuranceError(f"evidence references missing Markdown anchor: {reference}")
    if path.suffix.lower() in {".yml", ".yaml"}:
        job = re.compile(rf"^  {re.escape(fragment)}:\s*$", re.MULTILINE)
        if not job.search(path.read_text(encoding="utf-8")):
            raise AssuranceError(f"evidence references missing workflow job: {reference}")


def validate_evidence(root: Path, owner: str, item: Any) -> None:
    if not isinstance(item, dict):
        raise AssuranceError(f"{owner} must be an object")
    if set(item) - {"level", "evidence", "rationale"}:
        raise AssuranceError(f"{owner} contains unsupported keys: {sorted(set(item) - {'level', 'evidence', 'rationale'})}")
    level = item.get("level")
    if level not in LEVELS:
        raise AssuranceError(f"{owner}.level must be one of {sorted(LEVELS)}")
    evidence = item.get("evidence", [])
    if not isinstance(evidence, list) or any(not isinstance(entry, str) or not entry for entry in evidence):
        raise AssuranceError(f"{owner}.evidence must be a list of non-empty strings")
    rationale = item.get("rationale")
    if level == "not_applicable":
        if evidence or not isinstance(rationale, str) or not rationale.strip():
            raise AssuranceError(f"{owner} marked not_applicable requires a rationale and no evidence")
        return
    if not evidence:
        raise AssuranceError(f"{owner} at level {level} requires evidence")
    if rationale is not None and (not isinstance(rationale, str) or not rationale.strip()):
        raise AssuranceError(f"{owner}.rationale must be a non-empty string when present")
    for reference in evidence:
        path = evidence_path(root, reference)
        if not path.exists():
            raise AssuranceError(f"{owner} references missing evidence: {reference}")
        validate_fragment(path, reference)


def validate_documentation(root: Path, owner: str, item: Any) -> int:
    if not isinstance(item, dict) or set(item) != DOCUMENTATION_TYPES:
        raise AssuranceError(
            f"{owner}.documentation must define exactly "
            f"{sorted(DOCUMENTATION_TYPES)}"
        )
    count = 0
    for kind in sorted(DOCUMENTATION_TYPES):
        references = item[kind]
        if (
            not isinstance(references, list)
            or not references
            or any(not isinstance(entry, str) or not entry for entry in references)
        ):
            raise AssuranceError(
                f"{owner}.documentation.{kind} must be a non-empty list of Markdown references"
            )
        for reference in references:
            path = evidence_path(root, reference)
            if not path.exists():
                raise AssuranceError(
                    f"{owner}.documentation.{kind} references missing documentation: {reference}"
                )
            if path.suffix.lower() != ".md":
                raise AssuranceError(
                    f"{owner}.documentation.{kind} must reference Markdown: {reference}"
                )
            validate_fragment(path, reference)
            count += 1
    return count


def validate_document(document: Any, root: Path = ROOT) -> dict[str, int]:
    if not isinstance(document, dict) or document.get("schema") != 1:
        raise AssuranceError("feature assurance schema must be 1")
    declared_quality = set(document.get("quality_dimensions", []))
    declared_platforms = set(document.get("supported_platforms", []))
    if declared_quality != QUALITY_DIMENSIONS:
        raise AssuranceError(f"quality_dimensions must be exactly {sorted(QUALITY_DIMENSIONS)}")
    if declared_platforms != SUPPORTED_PLATFORMS:
        raise AssuranceError(f"supported_platforms must be exactly {sorted(SUPPORTED_PLATFORMS)}")

    features = document.get("features")
    if not isinstance(features, list) or not features:
        raise AssuranceError("features must be a non-empty list")

    ids: set[str] = set()
    claimed_components: set[str] = set()
    security_evidence: set[str] = set()
    performance_evidence: set[str] = set()
    evidence_count = 0
    documentation_count = 0
    for index, feature in enumerate(features):
        owner = f"features[{index}]"
        if not isinstance(feature, dict):
            raise AssuranceError(f"{owner} must be an object")
        expected_feature_keys = {
            "id",
            "title",
            "components",
            "documentation",
            "quality",
            "platforms",
        }
        if set(feature) != expected_feature_keys:
            raise AssuranceError(
                f"{owner} must define exactly {sorted(expected_feature_keys)}"
            )
        feature_id = feature.get("id")
        if not isinstance(feature_id, str) or not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", feature_id):
            raise AssuranceError(f"{owner}.id must be a lowercase kebab-case identifier")
        if feature_id in ids:
            raise AssuranceError(f"duplicate feature id: {feature_id}")
        ids.add(feature_id)
        title = feature.get("title")
        if not isinstance(title, str) or not title.strip():
            raise AssuranceError(f"{feature_id}.title must be non-empty")

        components = feature.get("components")
        if not isinstance(components, list) or not components:
            raise AssuranceError(f"{feature_id}.components must be non-empty")
        for component in components:
            if not isinstance(component, str) or not component or component != Path(component).as_posix().rstrip("/"):
                raise AssuranceError(f"{feature_id} has a non-canonical component: {component!r}")
            if not (root / component).exists():
                raise AssuranceError(f"{feature_id} claims missing component: {component}")
            claimed_components.add(component)

        documentation_count += validate_documentation(
            root, feature_id, feature.get("documentation")
        )

        quality = feature.get("quality")
        if not isinstance(quality, dict) or set(quality) != QUALITY_DIMENSIONS:
            raise AssuranceError(f"{feature_id}.quality must define exactly {sorted(QUALITY_DIMENSIONS)}")
        for dimension in sorted(QUALITY_DIMENSIONS):
            validate_evidence(root, f"{feature_id}.quality.{dimension}", quality[dimension])
            evidence_count += len(quality[dimension].get("evidence", []))
            evidence_paths = {
                evidence_path(root, reference).relative_to(root).as_posix()
                for reference in quality[dimension].get("evidence", [])
            }
            if dimension == "security":
                security_evidence.update(evidence_paths)
            elif dimension == "performance":
                performance_evidence.update(evidence_paths)

        platforms = feature.get("platforms")
        if not isinstance(platforms, dict) or set(platforms) != SUPPORTED_PLATFORMS:
            raise AssuranceError(f"{feature_id}.platforms must define exactly {sorted(SUPPORTED_PLATFORMS)}")
        for platform in sorted(SUPPORTED_PLATFORMS):
            validate_evidence(root, f"{feature_id}.platforms.{platform}", platforms[platform])
            evidence_count += len(platforms[platform].get("evidence", []))

    missing_members = workspace_members(root) - claimed_components
    if missing_members:
        raise AssuranceError(f"workspace members missing from feature assurance: {sorted(missing_members)}")
    missing_surfaces = REQUIRED_SURFACES - claimed_components
    if missing_surfaces:
        raise AssuranceError(f"repository surfaces missing from feature assurance: {sorted(missing_surfaces)}")

    discovered_benchmarks = benchmark_targets(root)
    missing_benchmarks = discovered_benchmarks - performance_evidence
    if missing_benchmarks:
        raise AssuranceError(f"benchmark targets missing from performance evidence: {sorted(missing_benchmarks)}")
    fuzz_targets = {
        path.relative_to(root).as_posix()
        for path in (root / "fuzz/fuzz_targets").glob("*.rs")
    }
    missing_fuzz = fuzz_targets - security_evidence
    if missing_fuzz:
        raise AssuranceError(f"fuzz targets missing from security evidence: {sorted(missing_fuzz)}")

    return {
        "features": len(features),
        "components": len(claimed_components),
        "evidence": evidence_count,
        "benchmarks": len(discovered_benchmarks),
        "fuzz_targets": len(fuzz_targets),
        "documentation": documentation_count,
    }


def load_and_validate(path: Path = DEFAULT_MATRIX, root: Path = ROOT) -> dict[str, int]:
    with path.open("r", encoding="utf-8") as source:
        return validate_document(json.load(source), root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--matrix", type=Path, default=DEFAULT_MATRIX)
    args = parser.parse_args()
    try:
        counts = load_and_validate(args.matrix)
    except (AssuranceError, OSError, json.JSONDecodeError, tomllib.TOMLDecodeError, KeyError) as error:
        print(f"feature assurance validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: feature assurance is traceable "
        f"(features={counts['features']}, components={counts['components']}, "
        f"benchmarks={counts['benchmarks']}, fuzz_targets={counts['fuzz_targets']}, "
        f"documentation={counts['documentation']}, evidence={counts['evidence']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
