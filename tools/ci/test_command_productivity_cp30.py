#!/usr/bin/env python3
"""Mutation tests for the active, non-activated CP3.0 compiler contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_command_productivity_cp30 as policy


class Cp30ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.Cp30Error):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        self.assertEqual(
            policy.validate_repository(),
            {"model_files": 5, "shells": 5, "tests": 13, "documents": 5},
        )

    def test_resource_expansion_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["limits"].__setitem__(
                "generated_file_bytes", 16_777_216
            )
        )

    def test_security_integrity_and_publication_drift_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["capabilities"].__setitem__("activation", True)
        )
        self.validate_mutation(
            lambda document: document["capabilities"].__setitem__("process", True)
        )
        self.validate_mutation(
            lambda document: document["observations"].__setitem__("tool_complete", False)
        )
        self.validate_mutation(
            lambda document: document["integrity"].__setitem__(
                "source_digest_recomputed", False
            )
        )
        self.validate_mutation(
            lambda document: document["publication"].__setitem__("profile_hook", True)
        )


    def test_shell_or_model_boundary_changes_are_rejected(self) -> None:
        self.validate_mutation(lambda document: document["shell_modes"].pop("fish"))
        self.validate_mutation(
            lambda document: document["model_files"].append(
                "apps/automexia-terminal/src/profile_writer.rs"
            )
        )

    def test_duplicate_contract_key_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.Cp30Error, "duplicate"):
                policy.load_contract(path)

    def test_projection_capability_primitive_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "projection.rs":
                source += "\nfn forbidden() { std::process::Command::new(\"sh\"); }\n"
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp30Error, "pure compiler boundary"):
                policy.validate_sources(self.contract)

    def test_missing_serializer_or_disabled_activation_marker_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "projection.rs":
                source = source.replace("render_cmd", "removed_cmd_serializer")
                source = source.replace("activation_enabled: false", "activation_enabled: true")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp30Error, "missing CP3.0 evidence"):
                policy.validate_sources(self.contract)

    def test_missing_native_capture_test_is_rejected(self) -> None:
        original = policy.bounded_text
        target = "available_native_shells_parse_and_capture_exact_typed_arguments"

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "quick_action_projection.rs":
                source = source.replace(f"fn {target}(", "fn removed_native_capture(")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp30Error, "required tests"):
                policy.validate_sources(self.contract)

    def test_linked_contract_is_rejected_where_supported(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target.json"
            linked = root / "linked.json"
            target.write_text("{}", encoding="utf-8")
            try:
                linked.symlink_to(target)
            except OSError:
                self.skipTest("symbolic links are unavailable")
            with self.assertRaises(policy.Cp30Error):
                policy.bounded_text(linked)


if __name__ == "__main__":
    unittest.main()
