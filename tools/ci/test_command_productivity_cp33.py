#!/usr/bin/env python3
"""Mutation tests for CP3.3 native imports and workspace task bridges."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_command_productivity_cp33 as policy


class Cp33ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.Cp33Error):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        self.assertEqual(
            policy.validate_repository(),
            {
                "native_sources": 6,
                "task_runners": 3,
                "commands": 6,
                "tests": 14,
                "documents": 9,
            },
        )

    def test_contract_inventories_cannot_drift(self) -> None:
        self.validate_mutation(lambda document: document["native_sources"].pop())
        self.validate_mutation(lambda document: document["task_runners"].pop())
        self.validate_mutation(lambda document: document["management_commands"].pop())
        self.validate_mutation(
            lambda document: document.__setitem__("status", "partially-done")
        )
        self.validate_mutation(
            lambda document: document["source_files"].append("apps/unreviewed_writer.rs")
        )

    def test_security_and_lifecycle_claims_cannot_weaken(self) -> None:
        for key, expected in policy.EXPECTED_SECURITY.items():
            self.validate_mutation(
                lambda document, key=key, expected=expected: document[
                    "security"
                ].__setitem__(key, not expected)
            )
        for key in policy.EXPECTED_LIFECYCLE:
            self.validate_mutation(
                lambda document, key=key: document["lifecycle"].__setitem__(
                    key, False
                )
            )

    def test_duplicate_contract_key_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.Cp33Error, "duplicate"):
                policy.load_contract(path)

    def test_parser_trust_runtime_and_ui_guards_cannot_disappear(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            replacements = {
                "imports.rs": ("parse_git_inventory", "removed_git_parser"),
                "workspace.rs": (
                    "open_existing_read_only",
                    "removed_read_only_trust_lookup",
                ),
                "worker.rs": (
                    "WORKSPACE_AUTHORIZATION_TTL",
                    "REMOVED_AUTHORIZATION_TTL",
                ),
                "action_surface.rs": (
                    "ensure_selected_workspace_action_authorized",
                    "removed_insertion_guard",
                ),
            }
            if path.name in replacements:
                source = source.replace(*replacements[path.name])
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp33Error, "missing CP3.3 evidence"):
                policy.validate_sources(self.contract)

    def test_dry_run_cas_and_lifecycle_dispatch_cannot_disappear(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.as_posix().endswith("/src/cli.rs"):
                source = source.replace(
                    'requires = "expected_revision"',
                    'requires = "removed_revision"',
                )
            if path.name == "native_import.rs":
                source = source.replace("replace_conflicts", "removed_conflict_guard")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp33Error, "missing CP3.3 evidence"):
                policy.validate_sources(self.contract)

    def test_test_benchmark_fuzz_and_nightly_evidence_cannot_disappear(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "quick_action_imports.rs":
                source = source.replace(
                    "duplicate_secret_control_and_oversized_inventories_fail_closed",
                    "removed_hostile_fixture_test",
                )
            if path.name == "quick_actions.rs":
                source = source.replace(
                    "quick_action_workspace_trust_verify",
                    "removed_trust_benchmark",
                )
            if path.name == "Cargo.toml" and path.parent.name == "fuzz":
                source = source.replace("quick_action_imports", "removed_import_fuzz")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaises(policy.Cp33Error):
                policy.validate_sources(self.contract)


if __name__ == "__main__":
    unittest.main()
