#!/usr/bin/env python3
"""Mutation tests for the CP1 native-completion activation contract."""

from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_command_productivity_cp1", ROOT / "tools/ci/check_command_productivity_cp1.py"
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load CP1 checker")
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)
CONTRACT = json.loads(
    (ROOT / "tests/fixtures/command-productivity/cp1-contract-v1.json").read_text(encoding="utf-8")
)


class Cp1PolicyTests(unittest.TestCase):
    def test_repository_contract_validates(self) -> None:
        counts = POLICY.validate_repository(ROOT)
        self.assertEqual(counts["shells"], 5)
        self.assertEqual(counts["providers"], 11)
        self.assertEqual(counts["activation_files"], 12)

    def test_missing_provider_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        del changed["providers"]["helm"]
        with self.assertRaisesRegex(POLICY.Cp1Error, "provider policy"):
            POLICY.validate_contract(changed)

    def test_increased_deadline_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["limits"]["provider_deadline_ms"] += 1
        with self.assertRaisesRegex(POLICY.Cp1Error, "resource ceilings"):
            POLICY.validate_contract(changed)

    def test_extra_activation_file_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["activation_files"].append("shell-integration/implicit-network.sh")
        with self.assertRaisesRegex(POLICY.Cp1Error, "activation allowlist"):
            POLICY.validate_contract(changed)

    def test_capability_escalation_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["capabilities"]["network"] = True
        with self.assertRaisesRegex(POLICY.Cp1Error, "capability boundary"):
            POLICY.validate_contract(changed)

    def test_dynamic_evaluation_is_rejected(self) -> None:
        with self.assertRaisesRegex(POLICY.Cp1Error, "dynamic evaluation"):
            POLICY.validate_adapter_text(
                "shell-integration/completion/bash/automexia-completion.bash",
                "eval provider-output; sha256 disabled complete -p source $file",
            )

    def test_adapter_without_digest_is_rejected(self) -> None:
        with self.assertRaisesRegex(POLICY.Cp1Error, "adapter controls"):
            POLICY.validate_adapter_text(
                "shell-integration/completion/zsh/automexia-completion.zsh",
                "disabled _comps source $file",
            )


if __name__ == "__main__":
    unittest.main()
