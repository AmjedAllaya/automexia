#!/usr/bin/env python3
"""Mutation tests for reviewed CP3.2 DevOps Quick Action packs."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_command_productivity_cp32 as policy


class Cp32ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.Cp32Error):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        self.assertEqual(policy.validate_repository(), {
            "providers": 11, "actions": 33, "commands": 4,
            "tests": 11, "documents": 6,
        })

    def test_provider_inventory_and_counts_cannot_drift(self) -> None:
        self.validate_mutation(lambda document: document["providers"].pop())
        self.validate_mutation(lambda document: document["action_ids"].pop())
        self.validate_mutation(lambda document: document["inventory"].__setitem__("actions", 32))

    def test_alias_and_capability_defaults_cannot_weaken(self) -> None:
        for key in (
            "aliases_disabled_by_default", "builtin_manifest_identity",
            "custom_overlays_user_provenance", "context_aliases_denied",
            "authentication_aliases_denied", "destructive_aliases_denied",
            "privileged_aliases_denied", "doctor_strictly_read_only",
            "compare_and_swap_writes", "enable_never_overwrites",
        ):
            self.validate_mutation(lambda document, key=key: document["security"].__setitem__(key, False))
        for key in ("provider_execution", "credential_reads", "network_access"):
            self.validate_mutation(lambda document, key=key: document["security"].__setitem__(key, True))

    def test_lifecycle_protections_cannot_be_removed(self) -> None:
        for key in policy.EXPECTED_LIFECYCLE:
            self.validate_mutation(lambda document, key=key: document["lifecycle"].__setitem__(key, False))

    def test_duplicate_contract_key_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.Cp32Error, "duplicate"):
                policy.load_contract(path)

    def test_missing_alias_guard_or_disabled_default_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "validation.rs":
                source = source.replace("validate_builtin_alias", "removed_alias_guard")
            if path.name == "packs.rs":
                source = source.replace("BuiltinDisabled", "RemovedDisabledDefault")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp32Error, "missing CP3.2 evidence"):
                policy.validate_sources(self.contract)

    def test_missing_health_update_or_cas_evidence_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "packs.rs":
                source = source.replace("plan_pack_update", "removed_update_planner")
            if path.name == "packs_cli.rs":
                source = source.replace("expected_revision", "removed_cas")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp32Error, "missing CP3.2 evidence"):
                policy.validate_sources(self.contract)

    def test_test_benchmark_and_fuzz_evidence_cannot_disappear(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "quick_action_packs.rs":
                source = source.replace(
                    "updates_preserve_overlays_and_explain_deprecations",
                    "removed_update_test",
                )
            if path.name == "quick_actions.rs":
                source = source.replace(
                    "quick_action_pack_registry_validate_11_33", "removed_pack_benchmark"
                )
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaises(policy.Cp32Error):
                policy.validate_sources(self.contract)


if __name__ == "__main__":
    unittest.main()