#!/usr/bin/env python3
"""Mutation tests for the CP1 native-completion activation contract."""

from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest import mock


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

    def test_zsh_noninteractive_security_contract_cannot_be_weakened(self) -> None:
        original_bounded_text = POLICY.bounded_text
        contract_path = ROOT / "tools/ci/test_zsh_integration.zsh"
        for removed in ("compinit -i -D", "automexia-test-insecure-canary"):
            with self.subTest(removed=removed):
                def mutated_bounded_text(path, limit, label):
                    text = original_bounded_text(path, limit, label)
                    if Path(path).resolve() == contract_path.resolve():
                        self.assertIn(removed, text)
                        return text.replace(removed, "")
                    return text

                with mock.patch.object(
                    POLICY, "bounded_text", side_effect=mutated_bounded_text
                ):
                    with self.assertRaisesRegex(POLICY.Cp1Error, "CP1 wiring"):
                        POLICY.validate_sources(ROOT)

    def test_bash_permission_probe_batching_cannot_be_weakened(self) -> None:
        original_bounded_text = POLICY.bounded_text
        contract_path = ROOT / "shell-integration/bash/automexia.bash"
        required = (
            "__automexia_alias_real_private_directories",
            "stat -c '%a %s'",
            "stat -f '%Lp %z'",
        )
        for removed in required:
            with self.subTest(removed=removed):
                def mutated_bounded_text(path, limit, label):
                    text = original_bounded_text(path, limit, label)
                    if Path(path).resolve() == contract_path.resolve():
                        self.assertIn(removed, text)
                        return text.replace(removed, "")
                    return text

                # The native latency regression proves the real effect; this
                # mutation guard prevents a source edit from silently restoring
                # the redundant subprocess probes that caused it.
                with mock.patch.object(
                    POLICY, "bounded_text", side_effect=mutated_bounded_text
                ):
                    with self.assertRaisesRegex(POLICY.Cp1Error, "CP1 wiring"):
                        POLICY.validate_sources(ROOT)


if __name__ == "__main__":
    unittest.main()
