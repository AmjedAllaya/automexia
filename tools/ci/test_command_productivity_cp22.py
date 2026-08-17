#!/usr/bin/env python3
"""Mutation tests for the active CP2.2 Quick Action contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock


sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_command_productivity_cp22 as policy  # noqa: E402


class Cp22ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.Cp22Error):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        counts = policy.validate_repository()
        self.assertEqual(counts["model_files"], 5)
        self.assertEqual(counts["tests"], 13)

    def test_resource_ceiling_expansion_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["limits"].__setitem__("query_bytes", 1_000_000)
        )

    def test_network_or_exact_launch_authority_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["capabilities"].__setitem__("network", True)
        )
        self.validate_mutation(
            lambda document: document["capabilities"].__setitem__("exact_launch", True)
        )

    def test_workspace_or_secret_activation_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document.__setitem__("workspace_activation", "automatic")
        )
        self.validate_mutation(
            lambda document: document.__setitem__("secret_expansion", "plaintext")
        )

    def test_precedence_changes_and_duplicate_sources_are_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["scope_precedence"].reverse()
        )
        self.validate_mutation(
            lambda document: document["model_files"].append(document["model_files"][0])
        )

    def test_linked_policy_input_is_rejected_where_supported(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target.json"
            linked = root / "linked.json"
            target.write_text("{}", encoding="utf-8")
            try:
                linked.symlink_to(target)
            except OSError:
                self.skipTest("symbolic links are unavailable")
            with self.assertRaises(policy.Cp22Error):
                policy.bounded_text(linked)

    def test_collapsed_user_scope_or_global_pending_slot_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "activation.rs":
                source = source.replace("LayerIdentity::ShellUser", "LayerIdentity::User")
            if path.name == "worker.rs":
                source = source.replace("latest_by_route", "latest")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaises(policy.Cp22Error):
                policy.validate_sources(self.contract)

    def test_secret_prompt_preflight_or_visible_health_removal_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "action_surface.rs":
                source = source.replace(
                    "unavailable_before_placeholder", "post_prompt_unavailable"
                )
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaises(policy.Cp22Error):
                policy.validate_sources(self.contract)


if __name__ == "__main__":
    unittest.main()
