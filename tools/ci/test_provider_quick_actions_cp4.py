#!/usr/bin/env python3
"""Mutation tests for the M13/F13/CP4 provider-action contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_provider_quick_actions_cp4 as policy


class CP4ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.CP4ContractError):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        self.assertEqual(
            policy.validate_repository(),
            {
                "providers": 7,
                "publication_outcomes": 3,
                "decisions": 9,
                "authorities_denied": 11,
                "tests": 25,
            },
        )

    def test_identity_limit_provider_decision_and_authority_drift_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["limits"].__setitem__("published_routes", 33)
        )
        self.validate_mutation(lambda document: document["providers"].pop())
        self.validate_mutation(lambda document: document["publication_outcomes"].pop())
        self.validate_mutation(lambda document: document["decisions"].pop())
        self.validate_mutation(
            lambda document: document["authorities"].__setitem__("process", True)
        )
        self.validate_mutation(
            lambda document: document.__setitem__("status", "activated")
        )

    def test_provider_composition_authority_drift_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.as_posix().endswith("quick_actions/providers.rs"):
                source += '\nfn widened() { std::process::Command::new("aws"); }\n'
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.CP4ContractError, "runtime authority"):
                policy.validate_sources(self.contract)

    def test_interactive_provider_adapter_import_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "worker.rs":
                source += "\nuse automexia_devops_aws::build_provider_quick_action;\n"
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.CP4ContractError, "interactive path"):
                policy.validate_sources(self.contract)

    def test_product_publication_bridge_removal_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "action_surface.rs":
                source = source.replace("provider_action_publication", "removed_bridge")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.CP4ContractError, "missing CP4 source"):
                policy.validate_sources(self.contract)

        def missing_final_sync(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            target = "self.sync_provider_actions_for_current_route()"
            if path.name == "action_surface.rs":
                index = source.rfind(target)
                source = source[:index] + "None" + source[index + len(target):]
            return source

        with mock.patch.object(
            policy, "bounded_text", side_effect=missing_final_sync
        ):
            with self.assertRaisesRegex(
                policy.CP4ContractError, "open and final authorization"
            ):
                policy.validate_sources(self.contract)

    def test_final_revalidation_test_removal_is_rejected(self) -> None:
        original = policy.bounded_text
        target = "provider_failure_states_are_actionable_and_never_claim_execution"

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "action_surface.rs":
                source = source.replace(f"fn {target}(", "fn removed_cp4_test(")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.CP4ContractError, "missing CP4 tests"):
                policy.validate_evidence(self.contract)

    def test_duplicate_contract_key_and_linked_evidence_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            duplicate = root / "duplicate.json"
            duplicate.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.CP4ContractError, "duplicate"):
                policy.load_contract(duplicate)
            target = root / "target.rs"
            linked = root / "linked.rs"
            target.write_text("struct Evidence;", encoding="utf-8")
            try:
                linked.symlink_to(target)
            except OSError:
                self.skipTest("symbolic links are unavailable")
            with self.assertRaises(policy.CP4ContractError):
                policy.bounded_text(linked)


if __name__ == "__main__":
    unittest.main(verbosity=2)