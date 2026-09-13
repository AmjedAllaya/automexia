#!/usr/bin/env python3
"""Mutation tests for the versioned Ghostty compatibility assurance gate."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("check_ghostty_compatibility.py")
SPEC = importlib.util.spec_from_file_location("automexia_ghostty_gate", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load Ghostty compatibility checker")
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class GhosttyCompatibilityGateTests(unittest.TestCase):
    def test_repository_contract_passes(self) -> None:
        result = CHECKER.validate_repository()
        self.assertEqual(result["version"], "1.3.1")
        self.assertEqual(result["fixture_artifacts"], 6)
        self.assertEqual(result["nightly_fuzz_targets"], 3)
        self.assertEqual(result["external_native_platforms"], 3)

    def test_duplicate_manifest_keys_are_rejected(self) -> None:
        with self.assertRaisesRegex(CHECKER.GhosttyCompatibilityError, "duplicate"):
            CHECKER.parse_json('{"schema_version": 1, "schema_version": 2}')

    def test_manifest_digest_mutation_is_detected(self) -> None:
        fixture = CHECKER.ROOT / CHECKER.FIXTURE_ROOT
        manifest = CHECKER.parse_json(
            (fixture / "manifest.json").read_text(encoding="utf-8")
        )
        manifest["artifacts"]["linux.json"]["sha256"] = "0" * 64
        with tempfile.TemporaryDirectory() as directory:
            copied = Path(directory)
            for name in manifest["artifacts"]:
                (copied / name).write_bytes((fixture / name).read_bytes())
            (copied / "manifest.json").write_text(
                json.dumps(manifest), encoding="utf-8"
            )
            with self.assertRaisesRegex(CHECKER.GhosttyCompatibilityError, "digest"):
                CHECKER.validate_fixture_manifest(copied)

    def test_gate_wiring_mutations_are_detected(self) -> None:
        valid = CHECKER.repository_sources()
        mutations = (
            ("xtask", "keybindings::verify()?;", ""),
            (
                "nightly",
                "- ghostty_migration",
                "# removed ghostty_migration",
            ),
            (
                "nightly",
                "-p automexia-keybindings --no-run --locked",
                "--no-run --locked",
            ),
            (
                "qa",
                '["cargo", "bench", "-p", "automexia-keybindings", "--bench", "registry"',
                '["cargo", "bench", "--bench", "registry"',
            ),
            (
                "qa",
                'os.environ.get("AUTOMEXIA_QA_GHOSTTY_EVIDENCE", "").strip()',
                '""',
            ),
            (
                "roadmap",
                "G0 — source lock",
                "G0 source lock",
            ),
            ("context", "pub fn clear_parked_topologies", "fn removed_clear"),
            (
                "inspector",
                "parked_controls_are_bounded_distinct_and_pointer_accessible",
                "removed_pointer_contract",
            ),
            (
                "screen_compatibility",
                "clear_parked_topologies()",
                "removed_clear()",
            ),
            (
                "screen",
                "if self.process_compatibility_inspector_key(key)",
                "if false",
            ),
        )
        for source, old, new in mutations:
            with self.subTest(source=source, token=old):
                changed = dict(valid)
                self.assertIn(old, changed[source])
                # The roadmap intentionally repeats phase names in its index and
                # detailed section. Mutate every matching contract anchor so a
                # surviving duplicate cannot make this deletion test a no-op.
                changed[source] = changed[source].replace(old, new)
                with self.assertRaises(CHECKER.GhosttyCompatibilityError):
                    CHECKER.validate_gate_sources(changed)


    def test_modal_input_precedence_mutation_is_detected(self) -> None:
        changed = CHECKER.repository_sources()
        source = changed["screen"]
        first = "if self.process_compatibility_inspector_key(key)"
        second = "if self.handle_image_preview_key(key)"
        self.assertLess(source.index(first), source.index(second))
        changed["screen"] = source.replace(first, "if moved_modal_guard", 1).replace(
            second, first, 1
        )
        with self.assertRaises(CHECKER.GhosttyCompatibilityError):
            CHECKER.validate_gate_sources(changed)


if __name__ == "__main__":
    unittest.main(verbosity=2)
