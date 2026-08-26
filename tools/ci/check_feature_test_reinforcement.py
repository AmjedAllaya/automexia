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
    "terminal-protocols-grid-history": {
        "needed_tests": (
            "boundary-only CMD D",
            "pre-epoch",
            "timezone or DST transitions",
        ),
        "verification_reinforcements": (
            "source prompt, following-prompt boundary",
            "no shell-provided timestamp text",
        ),
        "checker_reinforcements": (
            "local timezone conversion",
            "no-PTY side-effect coverage",
        ),
    },
    "renderer-fonts-responsive-ui": {
        "needed_tests": (
            "threshold-minus-one",
            "40 logical-pixel interaction targets",
            "zero PTY input",
            "WGPU and CPU",
            "fractional trackpad",
            "physical-to-logical scaling",
            "responsive reclamping",
            "full ISO local date and time",
            "compact date-time fallbacks",
        ),
        "verification_reinforcements": (
            "42-pixel header",
            "full-shelf RGBA",
            "ownership before pane selection",
            "1,024-row event bound",
            "persistent idle indicator",
            "painted command datetime label",
            "terminal cells, PTY bytes",
        ),
        "checker_reinforcements": (
            "184-pixel default tab cap",
            "modal event ownership",
            "no-fabrication behavior",
        ),
    }
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


def _markdown_anchors(path: Path) -> set[str]:
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
