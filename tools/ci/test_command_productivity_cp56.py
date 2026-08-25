#!/usr/bin/env python3
"""Mutation tests for the disabled CP5 source implementation boundary."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "ci"))

import check_command_productivity_cp56 as checker  # noqa: E402


class Cp56ImplementationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.texts = {
            relative: (ROOT / relative).read_text(encoding="utf-8")
            for relative in checker.REQUIRED
        }

    def test_repository_source_boundary_validates(self) -> None:
        counts = checker.validate_repository()
        self.assertEqual(counts["shell_adapters"], 4)
        self.assertEqual(counts["external_shell_gates"], 3)

    def test_missing_control_is_rejected(self) -> None:
        changed = deepcopy(self.texts)
        path = "apps/automexia-terminal/src/automexia/suggestions/platform/windows.rs"
        changed[path] = changed[path].replace("PIPE_REJECT_REMOTE_CLIENTS", "REMOVED")
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)

    def test_missing_lifecycle_transaction_is_rejected(self) -> None:
        changed = deepcopy(self.texts)
        path = "apps/automexia-terminal/src/automexia/suggestions/service.rs"
        changed[path] = changed[path].replace("lifecycle: Mutex<()>", "removed")
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)

    def test_network_transport_is_rejected(self) -> None:
        changed = deepcopy(self.texts)
        path = "apps/automexia-terminal/src/automexia/suggestions/service.rs"
        changed[path] += "\nTcpListener\n"
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)

    def test_shell_process_and_enter_authority_are_rejected(self) -> None:
        for mutation in ("Start-Process", "Invoke-WebRequest", "Enter"):
            changed = deepcopy(self.texts)
            path = "shell-integration/suggestions/powershell/automexia-suggestions.ps1"
            changed[path] += f"\n{mutation}\n"
            with self.assertRaises(checker.Cp56Error):
                checker.validate_texts(changed)

    def test_missing_preview_guard_is_rejected(self) -> None:
        changed = deepcopy(self.texts)
        path = "shell-integration/suggestions/fish/automexia-suggestions.fish"
        changed[path] = changed[path].replace("preview-disabled", "removed")
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)

    def test_shell_scaffolding_cannot_be_mislabeled_complete(self) -> None:
        changed = deepcopy(self.texts)
        path = "shell-integration/suggestions/README.md"
        changed[path] = changed[path].replace(
            "activation scaffolds, not a complete shell bridge", "complete bridge"
        )
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)


if __name__ == "__main__":
    unittest.main()
