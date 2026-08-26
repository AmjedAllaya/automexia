#!/usr/bin/env python3
"""Mutation tests for the redacted Ghostty native evidence contract."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import unittest


MODULE_PATH = Path(__file__).with_name("ghostty_native_evidence.py")
SPEC = importlib.util.spec_from_file_location("automexia_ghostty_native", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load Ghostty native evidence validator")
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)


COMMIT = "a" * 40


def run(platform: str) -> dict[str, object]:
    return {
        "platform": platform,
        "architecture": "x86_64",
        "application_sha256": "1" * 64,
        "package_sha256": "2" * 64,
        "fixture_kind": "windows-adapted" if platform == "windows" else "native",
        "fixture_sha256": "3" * 64,
        "scenarios": [
            {"id": scenario, "result": "pass", "duration_ms": 1}
            for scenario in sorted(VALIDATOR.SCENARIOS)
        ],
        "resources": {
            "peak_memory_bytes": 1,
            "peak_handles_or_fds": 1,
            "peak_processes": 1,
            "owned_processes_after": 0,
            "handles_or_fds_delta_after": 0,
            "threads_delta_after": 0,
            "parked_sessions_after_clear": 0,
            "duration_ms": 1,
        },
        "visual": {key: True for key in VALIDATOR.VISUAL_KEYS},
        "accessibility": {key: True for key in VALIDATOR.ACCESSIBILITY_KEYS},
    }


def manifest() -> dict[str, object]:
    return {
        "schema": 1,
        "evidence_kind": "ghostty-compatibility-1.3.1",
        "synthetic": False,
        "source_commit": COMMIT,
        "profile": {
            "id": "ghostty-1.3",
            "patch": "1.3.1",
            "upstream_commit": VALIDATOR.UPSTREAM_COMMIT,
            "default_profile": "automexia",
            "opt_in": True,
        },
        "runs": [run(platform) for platform in sorted(VALIDATOR.PLATFORMS)],
        "redaction": {
            "canaries_checked": 3,
            "canary_leaks": 0,
            "forbidden_fields_absent": True,
        },
    }


class GhosttyNativeEvidenceTests(unittest.TestCase):
    def test_complete_three_platform_contract_passes(self) -> None:
        result = VALIDATOR.validate_manifest(manifest(), COMMIT, True)
        self.assertEqual(result, {"platforms": 3, "scenarios": 24})

    def test_duplicate_json_key_is_rejected(self) -> None:
        with self.assertRaisesRegex(VALIDATOR.GhosttyNativeEvidenceError, "duplicate"):
            VALIDATOR.parse_manifest_bytes(b'{"schema":1,"schema":1}')

    def assert_rejected(self, mutation) -> None:
        value = copy.deepcopy(manifest())
        mutation(value)
        with self.assertRaises(VALIDATOR.GhosttyNativeEvidenceError):
            VALIDATOR.validate_manifest(value, COMMIT, True)

    def test_missing_platform_and_synthetic_evidence_are_rejected(self) -> None:
        self.assert_rejected(lambda value: value["runs"].pop())
        self.assert_rejected(lambda value: value.update(synthetic=True))

    def test_fixture_truth_and_cleanup_are_required(self) -> None:
        self.assert_rejected(
            lambda value: next(
                run for run in value["runs"] if run["platform"] == "windows"
            ).update(fixture_kind="native")
        )
        self.assert_rejected(
            lambda value: value["runs"][0]["resources"].update(owned_processes_after=1)
        )

    def test_visual_accessibility_and_redaction_cannot_be_erased(self) -> None:
        self.assert_rejected(lambda value: value["runs"][0]["visual"].update(hidpi=False))
        self.assert_rejected(
            lambda value: value["runs"][0]["accessibility"].update(native_at_reviewed=False)
        )
        self.assert_rejected(lambda value: value["redaction"].update(canary_leaks=1))


if __name__ == "__main__":
    unittest.main(verbosity=2)
