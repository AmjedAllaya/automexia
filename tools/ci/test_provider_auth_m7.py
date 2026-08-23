#!/usr/bin/env python3
"""Mutation tests for the provider-neutral M7/D6.0 contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_provider_auth_m7 as policy


class M7ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.M7ContractError):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        self.assertEqual(
            policy.validate_repository(),
            {"states": 19, "mutations": 7, "surfaces": 8, "tests": 12},
        )

    def test_limit_state_authority_and_isolation_drift_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["limits"].__setitem__("capsules", 65)
        )
        self.validate_mutation(lambda document: document["authentication_states"].pop())
        self.validate_mutation(
            lambda document: document["authorities"].__setitem__("process", True)
        )
        self.validate_mutation(lambda document: document["isolation_strategies"].pop())
        self.validate_mutation(lambda document: document.__setitem__("status", "shipped"))

    def test_runtime_authority_in_provider_model_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "provider_auth.rs":
                source += '\nfn forbidden() { std::process::Command::new("aws"); }\n'
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.M7ContractError, "runtime authority"):
                policy.validate_sources(self.contract)

    def test_passive_status_process_probe_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "context.rs":
                prefix, suffix = source.split("#[cfg(all(test", 1)
                source = prefix + '\nCommand::new("gcloud").spawn();\n#[cfg(all(test' + suffix
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.M7ContractError, "passive legacy"):
                policy.validate_sources(self.contract)

    def test_security_and_isolation_test_removal_is_rejected(self) -> None:
        original = policy.bounded_text
        target = "rebind_cancels_old_work_and_never_carries_context_across_capsules"

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "provider_auth_m7.rs":
                source = source.replace(f"fn {target}(", "fn removed_isolation_test(")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.M7ContractError, "missing M7 tests"):
                policy.validate_evidence(self.contract)

    def test_duplicate_contract_key_and_linked_contract_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            duplicate = root / "duplicate.json"
            duplicate.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.M7ContractError, "duplicate"):
                policy.load_contract(duplicate)
            target = root / "target.json"
            linked = root / "linked.json"
            target.write_text("{}", encoding="utf-8")
            try:
                linked.symlink_to(target)
            except OSError:
                self.skipTest("symbolic links are unavailable")
            with self.assertRaises(policy.M7ContractError):
                policy.bounded_text(linked)


if __name__ == "__main__":
    unittest.main()
