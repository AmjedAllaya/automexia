#!/usr/bin/env python3
"""Mutation tests for the M8-M12 multi-cloud stable-release contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_m8_m12_multicloud as policy


class M8M12ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = policy.load_contract()

    def validate_contract_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.MultiCloudContractError):
                policy.load_contract(path)

    def test_canonical_repository_contract_passes(self) -> None:
        self.assertEqual(
            policy.validate_repository(),
            {"sources": 9, "tests": 27, "limits": 17, "providers": 6, "benchmarks": 5},
        )

    def test_status_provider_limit_authority_and_external_gate_drift_fail_closed(self) -> None:
        self.validate_contract_mutation(
            lambda document: document.__setitem__("status", "fully-done")
        )
        self.validate_contract_mutation(lambda document: document["providers"].pop())
        self.validate_contract_mutation(
            lambda document: document["limits"].__setitem__("product_providers", 7)
        )
        self.validate_contract_mutation(
            lambda document: document["authorities"].__setitem__("execution_enabled", True)
        )
        self.validate_contract_mutation(lambda document: document["external_gates"].pop())

    def test_duplicate_contract_key_and_linked_contract_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            duplicate = root / "duplicate.json"
            duplicate.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(policy.MultiCloudContractError, "duplicate"):
                policy.load_contract(duplicate)
            target = root / "target.json"
            target.write_text("{}", encoding="utf-8")
            linked = root / "linked.json"
            try:
                linked.symlink_to(target)
            except OSError:
                self.skipTest("symbolic links are unavailable")
            with self.assertRaises(policy.MultiCloudContractError):
                policy.bounded_text(linked)

    def test_parser_isolation_and_nonactivation_weakening_are_rejected(self) -> None:
        original = policy.bounded_text
        cases = [
            ("lib.rs", "sso_region", "extensions/devops-aws"),
            ("lib.rs", "from_json_slice_without_duplicate_keys", "extensions/devops-azure"),
            ("lib.rs", "GcpAdapterErrorCode::DuplicateEntry", "extensions/devops-gcp"),
            ("implementation.rs", "MergeKeyPolicy::Error", "extensions/devops-kubernetes"),
            ("lib.rs", "--request-mode=off", "extensions/devops-teleport"),
            ("provider_transients.rs", "revalidate", "apps/automexia-terminal"),
            ("provider_transients.rs", "reserve_manager_root_with", "apps/automexia-terminal"),
            ("provider_transients.rs", "record.binding.capsule_revision != capsule_revision", "apps/automexia-terminal"),
            ("private_fs.rs", "windows_local_wide_path", "apps/automexia-terminal"),
            ("private_fs.rs", "segment == [b'.' as u16]", "apps/automexia-terminal"),
        ]
        for file_name, token, parent in cases:
            with self.subTest(file_name=file_name, parent=parent):
                def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
                    source = original(path, maximum)
                    normalized = path.as_posix()
                    if path.name == file_name and parent in normalized:
                        return source.replace(token, "removed-contract-token")
                    return source

                with mock.patch.object(policy, "bounded_text", side_effect=mutated):
                    with self.assertRaisesRegex(policy.MultiCloudContractError, "missing M8-M12 evidence"):
                        policy.validate_repository()

        def activation(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if "devops-openshift/src/implementation.rs" in path.as_posix():
                source = source.replace("execution_enabled: false", "execution_enabled: true")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=activation):
            with self.assertRaisesRegex(policy.MultiCloudContractError, "missing M8-M12 evidence"):
                policy.validate_repository()

    def test_required_real_path_regression_fuzz_parser_and_benchmark_removal_fail(self) -> None:
        original = policy.bounded_text
        removals = (
            ("lib.rs", "fn sso_login_uses_the_bound_sso_region_and_never_the_service_region("),
            ("multicloud_provider_inputs.rs", "automexia_devops_kubernetes::parse_private_transient_source"),
            ("Cargo.toml", "required-features = [\"multicloud-provider-inputs\"]"),
            ("provider.rs", "aws_public_config_128_profiles_and_sso_sessions"),
            ("provider_transients.rs", "fn long_local_paths_publish_revalidate_and_revoke_real_provider_transients("),
            ("private_fs.rs", "fn native_acl_path_conversion_rejects_relative_remote_and_dot_segments("),
        )
        for file_name, token in removals:
            with self.subTest(file_name=file_name, token=token):
                def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
                    source = original(path, maximum)
                    return source.replace(token, "removed-contract-token") if path.name == file_name else source

                with mock.patch.object(policy, "bounded_text", side_effect=mutated):
                    with self.assertRaises(policy.MultiCloudContractError):
                        policy.validate_repository()

    def test_provider_s1_coverage_removal_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
            source = original(path, maximum)
            if path.name == "s1-assurance-policy-v1.json":
                document = json.loads(source)
                for suite in document["required_suites"]:
                    coverage = suite.get("coverage", {})
                    for key in ("scenarios", "surfaces", "tasks"):
                        if "connection-hub-providers-review" in coverage.get(key, []):
                            coverage[key].remove("connection-hub-providers-review")
                source = json.dumps(document)
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.MultiCloudContractError, "provider S1"):
                policy.validate_repository()

    def test_ci_and_full_qa_wiring_removal_is_rejected(self) -> None:
        original = policy.bounded_text
        for file_name, token in (
            ("ci.yml", "python tools/ci/check_m8_m12_multicloud.py"),
            ("nightly.yml", "--no-default-features --features multicloud-provider-inputs"),
            ("qa.py", "tools/ci/test_m8_m12_multicloud.py"),
        ):
            with self.subTest(file_name=file_name):
                def mutated(path, maximum=policy.MAX_EVIDENCE_BYTES):
                    source = original(path, maximum)
                    return source.replace(token, "removed-multicloud-check") if path.name == file_name else source

                with mock.patch.object(policy, "bounded_text", side_effect=mutated):
                    with self.assertRaisesRegex(policy.MultiCloudContractError, "missing M8-M12 evidence"):
                        policy.validate_repository()


if __name__ == "__main__":
    unittest.main(verbosity=2)
