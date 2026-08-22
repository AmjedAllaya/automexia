#!/usr/bin/env python3
"""Mutation tests for the F1/D0-D3 session-launch review contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_session_launch_d0 as policy


class SessionLaunchD0ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.SessionLaunchD0Error):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        self.assertEqual(
            policy.validate_repository(),
            {
                "schema": 3,
                "scenarios": 19,
                "boundaries": 9,
                "sources": 12,
                "documents": 11,
                "production_enabled": 0,
            },
        )

    def test_activation_identity_grant_and_defaults_cannot_weaken(self) -> None:
        mutations = [
            lambda d: d["activation"].__setitem__("production_enabled", True),
            lambda d: d["activation"].__setitem__("required_protected_approvals", 1),
            lambda d: d["package_identity"].__setitem__("compatibility", "semver"),
            lambda d: d["package_identity"].__setitem__("digest_size_bytes", 0),
            lambda d: d["package_identity"].__setitem__("digest_source", "caller"),
            lambda d: d["package_identity"].__setitem__("unverified_denied", False),
            lambda d: d["package_identity"]["allowed_verification"].append("unverified"),
            lambda d: d["grant"].__setitem__("persistent_grants", True),
            lambda d: d["strict_defaults"].__setitem__("host_key_checking", "accept-new"),
            lambda d: d["strict_defaults"].__setitem__("forward_listener_scope", "any"),
            lambda d: d["strict_defaults"]["managed_options"].pop(),
            lambda d: d["strict_defaults"].__setitem__(
                "config_command_execution_during_discovery", True
            ),
            lambda d: d["executable_resolution"].__setitem__("path_lookup", True),
            lambda d: d["executable_resolution"]["wsl"].__setitem__(
                "production_enabled", True
            ),
        ]
        for mutate in mutations:
            self.validate_mutation(mutate)
        for key in self.contract["authority_ceiling"]:
            self.validate_mutation(
                lambda document, key=key: document["authority_ceiling"].__setitem__(
                    key, True
                )
            )
        for key, value in self.contract["strict_defaults"].items():
            if value is False:
                self.validate_mutation(
                    lambda document, key=key: document["strict_defaults"].__setitem__(
                        key, True
                    )
                )

    def test_scenarios_evidence_and_external_blocker_cannot_drift(self) -> None:
        self.validate_mutation(lambda d: d["native_scenarios"].pop())
        self.validate_mutation(lambda d: d["evidence"]["source"].pop())
        self.validate_mutation(lambda d: d["external_prerequisites"].pop())
        self.validate_mutation(lambda d: d.__setitem__("status", "fully-done"))

    def test_manual_boundary_and_native_harness_contract_cannot_drift(self) -> None:
        mutations = [
            lambda d: d["manual_baseline"].__setitem__("preserved", False),
            lambda d: d["manual_baseline"].__setitem__(
                "download_or_install_during_startup_or_launch", True
            ),
            lambda d: d["trust_boundaries"].pop(),
            lambda d: d["native_fixture_protocol"].__setitem__(
                "arbitrary_sleeps", True
            ),
            lambda d: d["native_fixture_protocol"]["timeouts_ms"].__setitem__(
                "case", 0
            ),
            lambda d: d["native_fixture_protocol"]["cleanup_invariants"].pop(),
            lambda d: d["native_fixture_protocol"]["redaction_surfaces"].pop(),
            lambda d: d["native_fixture_protocol"]["platform_activation"].__setitem__(
                "wsl", "scenario-outcome-required-before-any-managed-launch"
            ),
        ]
        for mutate in mutations:
            self.validate_mutation(mutate)

    def test_duplicate_contract_key_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.SessionLaunchD0Error, "duplicate"):
                policy.load_contract(path)

    def test_package_platform_and_activation_guards_cannot_disappear(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "launch_broker.rs":
                source = source.replace("pub struct PackageDigest", "removed package digest")
                source = source.replace('target_os = "macos"', 'target_os = "removed"')
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.SessionLaunchD0Error, "missing D0/D3 evidence"):
                policy.validate_sources(self.contract)

    def test_production_guard_and_runtime_authority_cannot_widen(self) -> None:
        original = policy.bounded_text

        def validate(rewrite) -> None:
            def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
                return rewrite(path, original(path, maximum))

            with mock.patch.object(policy, "bounded_text", side_effect=mutated):
                with self.assertRaises(policy.SessionLaunchD0Error):
                    policy.validate_sources(self.contract)

        validate(
            lambda path, source: source.replace(
                "pub mod launch_broker;",
                "#[cfg(test)]\npub mod launch_broker;",
            )
            if path.name == "mod.rs"
            else source
        )
        validate(
            lambda path, source: source.replace(
                "pub mod external_tool_runner;",
                "#[cfg(test)]\npub mod external_tool_runner;",
            )
            if path.name == "mod.rs"
            else source
        )
        validate(
            lambda path, source: source
            + '\nfn widened() { let _ = std::process::Command::new("ssh").spawn(); }\n'
            if path.name == "launch_broker.rs"
            else source
        )
        validate(
            lambda path, source: source
            + "\nfn bypass() { create_exact_pty(); }\n"
            if path.name == "external_tool_runner.rs"
            else source
        )

    def test_audit_forbidden_fields_are_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "launch_broker.rs":
                source = source.replace("pub extension_id: ExtensionId,", "pub argv: String,\n    pub extension_id: ExtensionId,", 1)
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.SessionLaunchD0Error, "forbidden"):
                policy.validate_sources(self.contract)


if __name__ == "__main__":
    unittest.main()
