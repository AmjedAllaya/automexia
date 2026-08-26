#!/usr/bin/env python3
"""Mutation tests for the accepted, non-activating D7/CP6 source boundary."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import unittest


MODULE_PATH = Path(__file__).with_name("check_ecosystem_d7_cp6.py")
SPEC = importlib.util.spec_from_file_location("automexia_ecosystem_d7_cp6", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load D7/CP6 ecosystem checker")
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class EcosystemD7Cp6Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = (CHECKER.ROOT / CHECKER.CONTRACT_PATH).read_text(encoding="utf-8")
        cls.document = CHECKER.parse_contract(cls.text)
        cls.acceptance_text = (
            CHECKER.ROOT / CHECKER.ACCEPTANCE_PATH
        ).read_text(encoding="utf-8")
        cls.acceptance = CHECKER.parse_contract(cls.acceptance_text)

    def assert_rejected(self, mutation) -> None:
        document = copy.deepcopy(self.document)
        mutation(document)
        with self.assertRaises(CHECKER.EcosystemContractError):
            CHECKER.validate_contract(document)

    def test_repository_contract_passes(self) -> None:
        counts = CHECKER.validate_repository()
        self.assertEqual(counts["threats"], 9)
        self.assertEqual(counts["limits"], 28)
        self.assertEqual(counts["external_gates"], 10)
        self.assertEqual(counts["source_files"], 18)

    def test_acceptance_receipt_cannot_gain_release_authority_or_drift_digest(self) -> None:
        release = copy.deepcopy(self.acceptance)
        release["release_authority"]["component_execution"] = True
        with self.assertRaises(CHECKER.EcosystemContractError):
            CHECKER.validate_acceptance(release)

        digest = copy.deepcopy(self.acceptance)
        digest["accepted_contract_canonical_sha256"] = "0" * 64
        with self.assertRaises(CHECKER.EcosystemContractError):
            CHECKER.validate_acceptance(digest)

        dependencies = copy.deepcopy(self.acceptance)
        dependencies["authorized_source"]["new_dependencies"].append("reqwest")
        with self.assertRaises(CHECKER.EcosystemContractError):
            CHECKER.validate_acceptance(dependencies)

    def test_acceptance_receipt_duplicate_key_is_rejected(self) -> None:
        duplicate = self.acceptance_text.replace(
            '"schema": 1,', '"schema": 1,\n  "schema": 1,', 1
        )
        with self.assertRaisesRegex(CHECKER.EcosystemContractError, "duplicate key"):
            CHECKER.parse_contract(duplicate)

    def test_duplicate_json_key_is_rejected(self) -> None:
        duplicate = self.text.replace('"schema": 1,', '"schema": 1,\n  "schema": 1,', 1)
        with self.assertRaisesRegex(CHECKER.EcosystemContractError, "duplicate key"):
            CHECKER.parse_contract(duplicate)

    def test_authority_cannot_be_activated(self) -> None:
        for key in (
            "accepted",
            "runtime_activation",
            "public_downloads",
            "public_sdk",
            "component_execution",
            "ai_provider_calls",
            "ai_tool_calls",
            "automatic_execution",
        ):
            with self.subTest(key=key):
                self.assert_rejected(lambda value, key=key: value["authority"].update({key: True}))

    def test_fallback_and_no_dependency_boundary_are_frozen(self) -> None:
        self.assert_rejected(lambda value: value["authority"].update(new_dependencies="wasmtime"))
        self.assert_rejected(lambda value: value["authority"].update(fallback="ecosystem-only"))

    def test_package_containment_cannot_be_weakened(self) -> None:
        self.assert_rejected(lambda value: value["package"].update(initial_ingress="network"))
        self.assert_rejected(lambda value: value["package"]["forbidden_payloads"].remove("symlink"))
        self.assert_rejected(lambda value: value["package"]["extraction"].remove("verify-before-extract"))

    def test_wasi_and_ambient_authority_remain_denied(self) -> None:
        self.assert_rejected(lambda value: value["sandbox"].update(default_wasi="full"))
        self.assert_rejected(lambda value: value["sandbox"]["forbidden_import_families"].remove("wasi:sockets"))
        self.assert_rejected(lambda value: value["sandbox"].update(execution_output="pty-enter"))

    def test_capability_grants_remain_exact_and_default_deny(self) -> None:
        self.assert_rejected(lambda value: value["capabilities"].update(default="allow"))
        self.assert_rejected(lambda value: value["capabilities"]["grant_binding"].remove("exact-package-digest"))
        self.assert_rejected(lambda value: value["capabilities"].update(read_implies_command=True))
        self.assert_rejected(lambda value: value["capabilities"]["initial_unavailable"].remove("command.execute"))

    def test_provenance_and_revocation_are_required(self) -> None:
        self.assert_rejected(lambda value: value["provenance"].update(verification_before_review=False))
        self.assert_rejected(lambda value: value["provenance"]["binds"].remove("current-revocation-state"))
        self.assert_rejected(lambda value: value["provenance"].update(signature_is_safety_claim=True))

    def test_distribution_remains_disabled_and_rollback_safe(self) -> None:
        self.assert_rejected(lambda value: value["distribution"].update(enabled=True))
        self.assert_rejected(lambda value: value["distribution"].update(startup_network=True))
        self.assert_rejected(lambda value: value["distribution"]["required_controls"].remove("rollback-freeze-and-mix-and-match-resistance"))

    def test_lifecycle_cleanup_and_fallback_are_frozen(self) -> None:
        self.assert_rejected(lambda value: value["lifecycle"].update(activation_now="allowed"))
        self.assert_rejected(lambda value: value["lifecycle"].update(kill_switch="restart-required"))
        self.assert_rejected(lambda value: value["lifecycle"]["preserve"].remove("user-files"))

    def test_ai_has_selected_input_only_and_never_executes(self) -> None:
        self.assert_rejected(lambda value: value["ai"].update(enabled=True))
        self.assert_rejected(lambda value: value["ai"]["allowed_input"].append("terminal-history"))
        self.assert_rejected(lambda value: value["ai"].update(tool_calls=True))
        self.assert_rejected(lambda value: value["ai"].update(automatic_execution=True))
        self.assert_rejected(lambda value: value["ai"].update(delivery="execute"))

    def test_threats_are_complete_and_have_verification_owners(self) -> None:
        self.assert_rejected(lambda value: value["security_threats"].pop())
        self.assert_rejected(lambda value: value["security_threats"][0].update(verification_owner=[]))
        self.assert_rejected(lambda value: value["security_threats"][7]["hostile_mutations"].remove("suggestion auto-runs"))

    def test_limits_cannot_expand_or_become_unbounded(self) -> None:
        self.assert_rejected(lambda value: value["limits"].update(bundle_bytes=67_108_864))
        self.assert_rejected(lambda value: value["limits"].update(linear_memory_bytes=0))
        self.assert_rejected(lambda value: value["limits"].update(concurrent_calls_global=64))
        self.assert_rejected(lambda value: value["limits"].update(selected_ai_input_bytes=1_048_576))

    def test_verification_domains_and_external_gates_cannot_be_erased(self) -> None:
        self.assert_rejected(lambda value: value["verification"].pop("privacy"))
        self.assert_rejected(lambda value: value.update(external_gates=[]))

    def test_contract_round_trip_is_stable(self) -> None:
        encoded = json.dumps(self.document, ensure_ascii=False, sort_keys=True)
        self.assertEqual(CHECKER.parse_contract(encoded), self.document)


if __name__ == "__main__":
    unittest.main(verbosity=2)
