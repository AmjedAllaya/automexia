#!/usr/bin/env python3
"""Mutation tests for persistent, explicitly opt-in CP3.1 aliases."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_command_productivity_cp31 as policy


class Cp31ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.Cp31Error):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        self.assertEqual(
            policy.validate_repository(),
            {"shells": 5, "commands": 11, "tests": 21, "native_files": 6, "documents": 5},
        )

    def test_limits_and_transaction_order_cannot_weaken(self) -> None:
        self.validate_mutation(
            lambda document: document["limits"].__setitem__("enabled_aliases", 1024)
        )
        self.validate_mutation(
            lambda document: document["publication"].__setitem__(
                "commit_order", "pointer-then-source"
            )
        )

    def test_security_and_native_precedence_cannot_weaken(self) -> None:
        self.validate_mutation(
            lambda document: document["security"].__setitem__("native_wins", False)
        )
        self.validate_mutation(
            lambda document: document["security"].__setitem__(
                "startup_provider_execution", True
            )
        )
        self.validate_mutation(
            lambda document: document["security"].__setitem__(
                "doctor_strictly_read_only", False
            )
        )

    def test_shell_command_or_source_boundary_change_is_rejected(self) -> None:
        self.validate_mutation(lambda document: document["shell_files"].pop("fish"))
        self.validate_mutation(lambda document: document["management_commands"].remove("doctor"))
        self.validate_mutation(
            lambda document: document["source_files"].append("apps/unreviewed_profile_writer.rs")
        )

    def test_duplicate_contract_key_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.Cp31Error, "duplicate"):
                policy.load_contract(path)

    def test_missing_transaction_or_read_only_evidence_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "aliases.rs":
                source = source.replace("prepare_transition", "removed_transition")
            if path.name == "store.rs":
                source = source.replace("open_existing_read_only", "removed_read_only")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp31Error, "missing CP3.1 evidence"):
                policy.validate_sources(self.contract)

    def test_required_test_inventory_and_compiler_identity_cannot_weaken(self) -> None:
        self.validate_mutation(
            lambda document: document["required_tests"].remove(
                "compiler_identity_mismatch_is_rejected"
            )
        )
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "automexia.bash":
                source = source.replace(
                    "generator=automexia-devops/0.4.0",
                    "generator=automexia-devops/unknown",
                )
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp31Error, "missing CP3.1 evidence"):
                policy.validate_sources(self.contract)

    def test_missing_native_collision_or_uninstall_proof_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "test_shell_integration.sh":
                source = source.replace("state=collision", "removed-collision")
            if path.name == "uninstall-unix.sh":
                source = source.replace("transaction.pending", "removed-journal")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp31Error, "missing CP3.1 evidence"):
                policy.validate_sources(self.contract)

    def reject_lock_mutation(self, relative: str, old: str, new: str) -> None:
        original = policy.bounded_text
        target = policy.ROOT / relative
        self.assertIn(old, original(target))

        def changed(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            return source.replace(old, new, 1) if path == target else source

        with mock.patch.object(policy, "bounded_text", side_effect=changed):
            with self.assertRaisesRegex(policy.Cp31Error, "advisory-lock ownership"):
                policy.validate_write_lock_lifecycle()

    def test_shared_lock_acquisition_and_retirement_cannot_be_inert_evidence(self) -> None:
        owner = "apps/automexia-terminal/src/automexia/private_fs.rs"
        for old, new in (
            ("file.try_lock()?;", "// file.try_lock()?;"),
            ("let _ = self.file.unlock();", "// let _ = self.file.unlock();"),
            ("file: File,", "pub file: File,"),
        ):
            with self.subTest(fragment=old):
                self.reject_lock_mutation(owner, old, new)

    def test_all_six_persistence_callers_keep_the_shared_guard(self) -> None:
        prefix = "apps/automexia-terminal/src/automexia/"
        for relative, value in (
            ("quick_actions/store.rs", "lock"),
            ("quick_actions/aliases.rs", "lock"),
            ("quick_actions/workspace.rs", "file"),
            ("connections/library.rs", "lock"),
            ("connections/receipts.rs", "lock"),
            ("preferences.rs", "lock"),
        ):
            with self.subTest(caller=relative):
                old = f"WriteLock::try_acquire({value})"
                self.reject_lock_mutation(prefix + relative, old,
                                          f"{value}.try_lock() /* {old} */")

    def test_prepared_alias_and_preference_transaction_do_not_release_early(self) -> None:
        self.reject_lock_mutation(
            "apps/automexia-terminal/src/automexia/quick_actions/aliases.rs",
            "_lock: WriteLock,", "_lock: File, // _lock: WriteLock,")
        self.reject_lock_mutation(
            "apps/automexia-terminal/src/automexia/preferences.rs",
            "let _lock = match WriteLock::try_acquire(lock)",
            "let _ = match WriteLock::try_acquire(lock)")

    def test_shared_lock_check_is_connected_and_uses_the_supplied_root(self) -> None:
        owner = policy.validate_write_lock_lifecycle
        with mock.patch.object(policy, "validate_write_lock_lifecycle", wraps=owner) as called:
            policy.validate_sources(self.contract)
        called.assert_called_once_with(policy.ROOT)
        original = policy.bounded_text
        with tempfile.TemporaryDirectory() as directory:
            other = Path(directory) / "checkout"
            paths = []

            def remapped(path, maximum=policy.MAX_POLICY_BYTES):
                paths.append(path)
                return original(policy.ROOT / path.relative_to(other), maximum)

            with mock.patch.object(policy, "bounded_text", side_effect=remapped):
                owner(other)
        self.assertEqual(len(paths), 7)
        self.assertTrue(all(path.is_relative_to(other) for path in paths))


if __name__ == "__main__":
    unittest.main()
