#!/usr/bin/env python3
"""Mutation tests for the immutable external GitHub Action trust policy."""

from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).resolve().parents[2] / ".github/scripts/check_action_pins.py"
SPEC = importlib.util.spec_from_file_location("automexia_action_pins", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load .github/scripts/check_action_pins.py")
PINS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PINS)


class ActionPinTests(unittest.TestCase):
    def validate(self, source: str) -> list[str]:
        with tempfile.TemporaryDirectory() as temporary:
            workflows = Path(temporary) / "workflows"
            workflows.mkdir()
            (workflows / "ci.yml").write_text(source, encoding="utf-8")
            return PINS.validate_workflows(workflows)

    def test_repository_workflows_pass(self) -> None:
        self.assertEqual(PINS.validate_workflows(PINS.WORKFLOWS), [])

    def test_allowlisted_full_sha_and_local_action_pass(self) -> None:
        failures = self.validate(
            "jobs:\n  test:\n    steps:\n"
            "      - uses: ./local-action\n"
            "      - uses: actions/checkout@0123456789abcdef0123456789abcdef01234567\n"
        )
        self.assertEqual(failures, [])

    def test_compiler_cache_action_accepts_only_the_reviewed_commit(self) -> None:
        reviewed = (
            "mozilla-actions/sccache-action@"
            "fc920bf0ec8de6ee65d409111f7ec508035751ba"
        )
        self.assertEqual(self.validate(f"steps:\n  - uses: {reviewed}\n"), [])

        # Compiler caches feed executable build inputs back into later jobs.
        # A repository allowlist alone must not silently authorize a different
        # action implementation at another otherwise-valid full SHA.
        unreviewed = (
            "mozilla-actions/sccache-action@"
            "1111111111111111111111111111111111111111"
        )
        failures = self.validate(f"steps:\n  - uses: {unreviewed}\n")
        self.assertTrue(any("reviewed commit" in failure for failure in failures), failures)

    def test_python_bootstrap_accepts_only_the_reviewed_commit(self) -> None:
        action = 'actions/setup-python@ece7cb06caefa5fff74198d8649806c4678c61a1'
        self.assertEqual(self.validate(f'steps:\n  - uses: {action}\n'), [])
        for reference in ('actions/setup-python@v6', 'actions/setup-python@' + '1' * 40):
            self.assertTrue(self.validate(f'steps:\n  - uses: {reference}\n'))

    def test_mutable_docker_and_unreviewed_actions_fail(self) -> None:
        cases = (
            ("actions/checkout@v4", "not pinned"),
            ("docker://example.invalid/tool:latest", "docker://"),
            (
                "unreviewed/example@0123456789abcdef0123456789abcdef01234567",
                "not allowlisted",
            ),
        )
        for reference, expected in cases:
            with self.subTest(reference=reference):
                failures = self.validate(f"steps:\n  - uses: {reference}\n")
                self.assertTrue(any(expected in failure for failure in failures), failures)

    def test_linked_and_oversized_workflow_inputs_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workflows = root / "workflows"
            workflows.mkdir()
            external = root / "outside.yml"
            external.write_text("steps: []\n", encoding="utf-8")
            linked = workflows / "ci.yml"
            os.link(external, linked)
            # The content is valid on purpose; ownership, not YAML syntax, must
            # reject an externally mutable hard-linked workflow.
            failures = PINS.validate_workflows(workflows)
            self.assertTrue(any("regular unlinked" in failure for failure in failures))

        with tempfile.TemporaryDirectory() as temporary:
            workflows = Path(temporary) / "workflows"
            workflows.mkdir()
            (workflows / "ci.yml").write_text("x" * 33, encoding="utf-8")
            failures = PINS.validate_workflows(workflows, max_file_bytes=32)
            self.assertTrue(any("byte limit" in failure for failure in failures))


if __name__ == "__main__":
    unittest.main(verbosity=2)
