#!/usr/bin/env python3
"""Mutation tests for the M6/F6 declarative-workspace contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_m6_workspaces as policy


class M6ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = policy.load_contract()

    def validate_contract_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.M6ContractError):
                policy.load_contract(path)

    def test_canonical_repository_contract_passes(self) -> None:
        self.assertEqual(
            policy.validate_repository(),
            {"sources": 7, "tests": 19, "limits": 10, "s1_surfaces": 3},
        )

    def test_status_limit_authority_and_activation_drift_fail_closed(self) -> None:
        self.validate_contract_mutation(
            lambda document: document.__setitem__("status", "fully-done")
        )
        self.validate_contract_mutation(
            lambda document: document["limits"].__setitem__("broadcast_targets", 51)
        )
        self.validate_contract_mutation(
            lambda document: document["authorities"].__setitem__("pty", True)
        )
        self.validate_contract_mutation(
            lambda document: document["activation_blockers"].pop()
        )

    def test_duplicate_contract_key_and_linked_contract_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            duplicate = root / "duplicate.json"
            duplicate.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.M6ContractError, "duplicate"):
                policy.load_contract(duplicate)
            target = root / "target.json"
            target.write_text("{}", encoding="utf-8")
            linked = root / "linked.json"
            try:
                linked.symlink_to(target)
            except OSError:
                self.skipTest("symbolic links are unavailable")
            with self.assertRaises(policy.M6ContractError):
                policy.bounded_text(linked)

    def test_security_lifecycle_or_product_owner_removal_is_rejected(self) -> None:
        cases = [
            ("strict_json.rs", "duplicate JSON object member name"),
            ("automation.rs", "validate_reviewed_recipe_run"),
            ("workspace.rs", "expire_pending_targets"),
            ("workspaces.rs", "required_profiles.remove"),
        ]
        original = policy.bounded_text
        for file_name, token in cases:
            with self.subTest(file_name=file_name):
                def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
                    source = original(path, maximum)
                    return source.replace(token, "removed-contract-token") if path.name == file_name else source

                with mock.patch.object(policy, "bounded_text", side_effect=mutated):
                    with self.assertRaisesRegex(policy.M6ContractError, "missing M6 evidence"):
                        policy.validate_repository()

    def test_capability_injection_and_activation_weakening_are_rejected(self) -> None:
        original = policy.bounded_text

        def capability(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "workspace.rs":
                source += '\nfn forbidden() { std::process::Command::new("ssh"); }\n'
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=capability):
            with self.assertRaisesRegex(policy.M6ContractError, "capability-free"):
                policy.validate_repository()

        def activation(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "workspaces.rs":
                source = source.replace(
                    "pub const fn execution_enabled(&self) -> bool {\n        false",
                    "pub const fn execution_enabled(&self) -> bool {\n        true",
                )
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=activation):
            with self.assertRaisesRegex(policy.M6ContractError, "missing M6 evidence"):
                policy.validate_repository()

    def test_required_real_path_regression_removal_is_rejected(self) -> None:
        original = policy.bounded_text
        target = "product_restore_selects_only_profiles_referenced_by_the_workspace"

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "m6_workspace_product.rs":
                source = source.replace(f"fn {target}(", "fn removed_large_library_regression(")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.M6ContractError, "missing M6 tests"):
                policy.validate_repository()

    def test_large_library_boundary_weakening_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "m6_workspace_product.rs":
                source = source.replace(
                    "0..(MAX_PROFILES - 1)",
                    "0..=MAX_WORKSPACE_CONNECTIONS",
                )
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.M6ContractError, "missing M6 evidence"):
                policy.validate_repository()

    def test_s1_workspace_coverage_removal_is_rejected(self) -> None:
        original = policy.bounded_text

        def removed_accessibility_suite(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "s1-assurance-policy-v1.json":
                document = json.loads(source)
                document["required_suites"] = [
                    suite
                    for suite in document["required_suites"]
                    if suite["domain"] != "accessibility"
                ]
                source = json.dumps(document)
            return source

        with mock.patch.object(
            policy, "bounded_text", side_effect=removed_accessibility_suite
        ):
            with self.assertRaisesRegex(policy.M6ContractError, "missing required M6 suites"):
                policy.validate_repository()

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "s1-assurance-policy-v1.json":
                document = json.loads(source)
                suite = next(item for item in document["required_suites"] if item["domain"] == "visual")
                suite["coverage"]["surfaces"].remove("connection-hub-workspace-broadcast")
                source = json.dumps(document)
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.M6ContractError, "visual S1"):
                policy.validate_repository()

        def weakened_capture_count(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "s1-assurance-policy-v1.json":
                document = json.loads(source)
                suite = next(item for item in document["required_suites"] if item["domain"] == "visual")
                suite["coverage"]["capture_count"] = 9_503
                source = json.dumps(document)
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=weakened_capture_count):
            with self.assertRaisesRegex(policy.M6ContractError, "capture count"):
                policy.validate_repository()

    def test_ci_and_full_qa_wiring_removal_is_rejected(self) -> None:
        original = policy.bounded_text
        for file_name, token in (
            ("ci.yml", "python tools/ci/check_m6_workspaces.py"),
            ("qa.py", "tools/ci/test_m6_workspaces.py"),
        ):
            with self.subTest(file_name=file_name):
                def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
                    source = original(path, maximum)
                    return source.replace(token, "removed-m6-check") if path.name == file_name else source

                with mock.patch.object(policy, "bounded_text", side_effect=mutated):
                    with self.assertRaisesRegex(policy.M6ContractError, "missing M6 evidence"):
                        policy.validate_repository()


if __name__ == "__main__":
    unittest.main(verbosity=2)
