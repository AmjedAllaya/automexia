#!/usr/bin/env python3
"""Mutation tests for the capability-free F2/D5.0 contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_connection_hub_f2 as policy


class F2ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.F2ContractError):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        self.assertEqual(
            policy.validate_repository(),
            {
                "models": 6,
                "tests": 22,
                "providers": 10,
                "auth_states": 14,
                "layouts": 4,
                "fixtures": 4,
            },
        )

    def test_limit_or_authority_expansion_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["limits"].__setitem__("steps_per_recipe", 65)
        )
        self.validate_mutation(
            lambda document: document["authorities"].__setitem__("process", True)
        )
        self.validate_mutation(
            lambda document: document["authorities"].__setitem__("network", True)
        )

    def test_provider_state_layout_or_status_drift_is_rejected(self) -> None:
        self.validate_mutation(lambda document: document["providers"].pop())
        self.validate_mutation(lambda document: document["authentication_states"].pop())
        self.validate_mutation(lambda document: document["result_states"].pop())
        self.validate_mutation(lambda document: document["content_states"].pop())
        self.validate_mutation(lambda document: document["layout_fixtures"].pop())
        self.validate_mutation(lambda document: document.__setitem__("status", "shipped"))

    def test_capability_primitive_in_model_sources_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "planner.rs":
                source += '\nfn forbidden() { std::process::Command::new("ssh"); }\n'
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.F2ContractError, "capability-free"):
                policy.validate_sources(self.contract)

    def test_disabled_execution_or_modal_inertness_mutation_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "connection_hub.rs":
                source = source.replace("pty_resize_requested: false", "pty_resize_requested: true")
                source = source.replace("background_inert: true", "background_inert: false")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.F2ContractError, "missing F2 evidence"):
                policy.validate_sources(self.contract)

    def test_required_security_or_accessibility_test_removal_is_rejected(self) -> None:
        original = policy.bounded_text
        target = "hostile_unknown_secret_command_and_bidi_fields_fail_closed"

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "connection_planning.rs":
                source = source.replace(f"fn {target}(", "fn removed_security_test(")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.F2ContractError, "missing F2 tests"):
                policy.validate_sources(self.contract)

    def test_duplicate_contract_key_and_linked_contract_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            duplicate = root / "duplicate.json"
            duplicate.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.F2ContractError, "duplicate"):
                policy.load_contract(duplicate)
            target = root / "target.json"
            linked = root / "linked.json"
            target.write_text("{}", encoding="utf-8")
            try:
                linked.symlink_to(target)
            except OSError:
                self.skipTest("symbolic links are unavailable")
            with self.assertRaises(policy.F2ContractError):
                policy.bounded_text(linked)


if __name__ == "__main__":
    unittest.main()
