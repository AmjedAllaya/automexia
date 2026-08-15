#!/usr/bin/env python3
"""Mutation tests for the CP0 command-productivity policy checker."""

from __future__ import annotations

from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_command_productivity", ROOT / "tools/ci/check_command_productivity.py"
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_command_productivity.py")
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)


def load_fixture(name: str) -> dict:
    path = ROOT / "tests/fixtures/command-productivity" / name
    return json.loads(path.read_text(encoding="utf-8"))


class CommandProductivityPolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = load_fixture("cp0-contract-v1.json")
        self.threats = load_fixture("cp0-threats-v1.json")

    def test_repository_contract_validates(self) -> None:
        counts = POLICY.validate_repository(ROOT)
        self.assertEqual(counts["shells"], 5)
        self.assertEqual(counts["providers"], 11)
        self.assertEqual(counts["threats"], 16)

    def test_missing_shell_is_rejected(self) -> None:
        mutated = deepcopy(self.contract)
        mutated["shells"] = [
            shell for shell in mutated["shells"] if shell["id"] != "fish"
        ]
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "shell matrix"):
            POLICY.validate_contract(mutated)

    def test_weakened_precedence_is_rejected(self) -> None:
        mutated = deepcopy(self.contract)
        mutated["precedence"][0:2] = ["capsule", "session"]
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "precedence"):
            POLICY.validate_contract(mutated)

    def test_provider_keystroke_execution_is_rejected(self) -> None:
        mutated = deepcopy(self.contract)
        mutated["providers"][0]["keystroke"] = True
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "per keystroke"):
            POLICY.validate_contract(mutated)

    def test_resource_ceiling_increase_is_rejected(self) -> None:
        mutated = deepcopy(self.contract)
        mutated["limits"]["source_file_bytes"] += 1
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "resource ceilings"):
            POLICY.validate_contract(mutated)

    def test_duplicate_threat_is_rejected(self) -> None:
        mutated = deepcopy(self.threats)
        mutated["threats"].append(deepcopy(mutated["threats"][0]))
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "duplicate id"):
            POLICY.validate_threats(mutated)

    def test_missing_threat_is_rejected(self) -> None:
        mutated = deepcopy(self.threats)
        mutated["threats"].pop()
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "threat catalog"):
            POLICY.validate_threats(mutated)

    def test_unaccepted_adr_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in (
                "docs/adr/0015-shell-native-completion-and-typed-quick-actions.md",
                "docs/COMMAND-PRODUCTIVITY.md",
                "docs/COMMAND-PRODUCTIVITY-COMPATIBILITY.md",
                "docs/COMMAND-PRODUCTIVITY-THREAT-MODEL.md",
                "docs/STABILIZATION-ROADMAP.md",
            ):
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                content = (ROOT / relative).read_text(encoding="utf-8")
                destination.write_text(
                    content.replace("- Status: Accepted for v0.5", "- Status: Proposed for v0.5"),
                    encoding="utf-8",
                )
            with self.assertRaisesRegex(POLICY.CommandProductivityError, "Status"):
                POLICY.validate_documents(root)

    def test_shell_startup_provider_hook_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "shell-integration").mkdir(parents=True)
            (root / "shell-integration/automexia.sh").write_text(
                "docker completion bash\n", encoding="utf-8"
            )
            for relative in (
                "apps/automexia-terminal/src/renderer",
                "apps/automexia-terminal/src/screen",
                "rio-vt/src",
                "teletypewriter/src",
            ):
                path = root / relative
                path.mkdir(parents=True)
                (path / "safe.rs").write_text("fn safe() {}\n", encoding="utf-8")
            test = root / "tools/ci/test_shell_integration.ps1"
            test.parent.mkdir(parents=True)
            test.write_text(
                "alias docker alias kubectl function ax function kgp\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(POLICY.CommandProductivityError, "shell startup"):
                POLICY.validate_pre_activation(root)

    def test_terminal_grid_command_inference_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "shell-integration").mkdir(parents=True)
            (root / "shell-integration/safe.sh").write_text("true\n", encoding="utf-8")
            for relative in (
                "apps/automexia-terminal/src/renderer",
                "apps/automexia-terminal/src/screen",
                "rio-vt/src",
                "teletypewriter/src",
            ):
                path = root / relative
                path.mkdir(parents=True)
                content = (
                    "fn infer() { let _ = QuickAction; let _ = terminal.grid; }\n"
                    if relative.endswith("renderer")
                    else "fn safe() {}\n"
                )
                (path / "source.rs").write_text(content, encoding="utf-8")
            test = root / "tools/ci/test_shell_integration.ps1"
            test.parent.mkdir(parents=True)
            test.write_text(
                "alias docker alias kubectl function ax function kgp\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(POLICY.CommandProductivityError, "grid inference"):
                POLICY.validate_pre_activation(root)

    def test_missing_ci_wiring_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = {
                "tools/ci/validate_repository.py": (
                    'validate_command_productivity()\n'
                    '"command productivity CP0"\n'
                ),
                "tools/xtask/src/main.rs": (
                    'run_python("tools/ci/check_command_productivity.py")?;\n'
                ),
                ".github/workflows/ci.yml": "name: policy\n",
            }
            for relative, content in files.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError,
                "test_command_productivity.py",
            ):
                POLICY.validate_wiring(root)


if __name__ == "__main__":
    unittest.main()
