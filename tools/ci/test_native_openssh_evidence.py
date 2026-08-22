#!/usr/bin/env python3
"""Mutation tests for native OpenSSH release-evidence validation."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import native_openssh_evidence as evidence


ZERO_HASH = "0" * 64


def valid_manifest(*, synthetic: bool = True) -> dict[str, object]:
    return {
        "schema": 1,
        "evidence_kind": "native-openssh-release",
        "synthetic": synthetic,
        "contract_sha256": evidence.contract_sha256(),
        "source_commit": "1" * 40,
        "platform": "windows",
        "architecture": "x86_64",
        "application": {
            "version": "0.4.0",
            "binary_sha256": ZERO_HASH,
            "package_sha256": ZERO_HASH,
        },
        "fixture": {
            "fixture_sha256": ZERO_HASH,
            "server_config_sha256": ZERO_HASH,
            "known_hosts_seed_sha256": ZERO_HASH,
            "random_seed_sha256": ZERO_HASH,
            "loopback_only": True,
            "internet_disabled": True,
            "private_workspace": True,
            "workspace_removed_after": True,
        },
        "openssh": {
            "client_version": "OpenSSH_synthetic_10.5 test-only",
            "server_version": "OpenSSH_synthetic_10.5 test-only",
            "security_policy": "platform-supported-current-advisory-review",
            "post_quantum_kex": True,
            "weak_crypto_warning": True,
            "agent_session_binding_restricted_key": True,
        },
        "scenarios": [
            {"id": scenario, "result": "pass", "duration_ms": 1}
            for scenario in evidence.SCENARIO_IDS
        ],
        "lifecycle": {
            "session_counts": [1, 10, 50],
            "connect_latency_p95_ms": 1,
            "cancellation_latency_p95_ms": 1,
            "shutdown_latency_p95_ms": 1,
            "peak_cpu_millicores": 1,
            "idle_cpu_millicores_after": 0,
            "peak_memory_bytes": 1,
            "peak_handles": 1,
            "peak_processes": 1,
            "peak_ptys": 1,
            "peak_listeners": 1,
            "peak_tunnels": 1,
            "peak_tasks": 1,
            "peak_routes": 1,
            "peak_cache_bytes": 0,
            "peak_log_bytes": 0,
            "peak_storage_bytes": 0,
            "owned_children_after": 0,
            "owned_ptys_after": 0,
            "owned_listeners_after": 0,
            "owned_tunnels_after": 0,
            "owned_tasks_after": 0,
            "stale_routes_after": 0,
            "open_handles_delta_after": 0,
            "temporary_secret_files_after": 0,
            "temporary_workspace_bytes_after": 0,
        },
        "manual_baseline": {
            "client_sha256_before": ZERO_HASH,
            "client_sha256_after": ZERO_HASH,
            "user_config_sha256_before": ZERO_HASH,
            "user_config_sha256_after": ZERO_HASH,
            "manual_ssh_before": True,
            "manual_ssh_after": True,
            "disable_preserved": True,
            "uninstall_preserved": True,
        },
        "redaction": {
            "canaries_checked": 1,
            "canary_leaks": 0,
            "forbidden_fields_absent": True,
        },
    }


class NativeOpenSshEvidenceTests(unittest.TestCase):
    def validate(self, document: dict[str, object], *, allow_synthetic: bool = True):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            return evidence.validate_manifest(path, allow_synthetic=allow_synthetic)

    def reject(self, mutate) -> None:
        document = valid_manifest()
        mutate(document)
        with self.assertRaises(evidence.NativeOpenSshEvidenceError):
            self.validate(document)

    def test_synthetic_repository_fixture_and_generated_contract_pass(self) -> None:
        self.assertEqual(
            self.validate(valid_manifest()),
            {
                "platform": "windows",
                "scenarios": 23,
                "sessions": 61,
                "synthetic": 1,
            },
        )
        self.assertEqual(
            evidence.validate_repository_contract(),
            {"scenarios": 23, "platforms": 3},
        )

    def test_synthetic_wsl_and_unbound_evidence_cannot_release(self) -> None:
        with self.assertRaisesRegex(
            evidence.NativeOpenSshEvidenceError, "synthetic evidence"
        ):
            self.validate(valid_manifest(), allow_synthetic=False)
        self.reject(lambda document: document.__setitem__("platform", "wsl"))
        self.reject(
            lambda document: document.__setitem__("contract_sha256", "f" * 64)
        )

    def test_release_evidence_binds_current_source_build_fixture_and_security(self) -> None:
        document = valid_manifest(synthetic=False)
        expected_commit = "2" * 40
        document["source_commit"] = expected_commit
        document["application"]["binary_sha256"] = "2" * 64
        document["application"]["package_sha256"] = "3" * 64
        document["fixture"]["fixture_sha256"] = "4" * 64
        document["fixture"]["server_config_sha256"] = "5" * 64
        document["fixture"]["known_hosts_seed_sha256"] = "6" * 64
        document["fixture"]["random_seed_sha256"] = "7" * 64
        document["openssh"]["client_version"] = "OpenSSH_10.5"
        document["openssh"]["server_version"] = "OpenSSH_10.5"
        with mock.patch.object(
            evidence, "current_source_commit", return_value=expected_commit
        ):
            self.assertEqual(
                self.validate(document, allow_synthetic=False)["synthetic"],
                0,
            )

        mutations = [
            lambda value: value.__setitem__("source_commit", "f" * 40),
            lambda value: value["application"].__setitem__(
                "binary_sha256", "not-a-hash"
            ),
            lambda value: value["fixture"].__setitem__("loopback_only", False),
            lambda value: value["fixture"].__setitem__(
                "workspace_removed_after", False
            ),
            lambda value: value["openssh"].__setitem__("post_quantum_kex", False),
            lambda value: value["openssh"].__setitem__(
                "agent_session_binding_restricted_key", False
            ),
        ]
        for mutate in mutations:
            changed = copy.deepcopy(document)
            mutate(changed)
            with mock.patch.object(
                evidence, "current_source_commit", return_value=expected_commit
            ):
                with self.assertRaises(evidence.NativeOpenSshEvidenceError):
                    self.validate(changed, allow_synthetic=False)

    def test_current_source_commit_rejects_a_dirty_tracked_tree(self) -> None:
        revision = evidence.subprocess.CompletedProcess(
            args=[], returncode=0, stdout=("2" * 40 + "\n").encode("ascii")
        )
        dirty = evidence.subprocess.CompletedProcess(
            args=[], returncode=1, stdout=b""
        )
        with mock.patch.object(
            evidence.subprocess, "run", side_effect=[revision, dirty]
        ):
            with self.assertRaisesRegex(
                evidence.NativeOpenSshEvidenceError, "tracked source tree"
            ):
                evidence.current_source_commit()

    def test_scenarios_and_resource_cleanup_are_exact(self) -> None:
        mutations = [
            lambda document: document["scenarios"].pop(),
            lambda document: document["scenarios"][0].__setitem__("result", "fail"),
            lambda document: document["scenarios"][0].__setitem__(
                "duration_ms", evidence.MAX_SCENARIO_DURATION_MS + 1
            ),
            lambda document: document["lifecycle"].__setitem__(
                "session_counts", [1, 10]
            ),
            lambda document: document["lifecycle"].__setitem__(
                "owned_tunnels_after", 1
            ),
            lambda document: document["lifecycle"].__setitem__(
                "peak_cpu_millicores", 4_001
            ),
            lambda document: document["lifecycle"].__setitem__(
                "connect_latency_p95_ms", 15_001
            ),
            lambda document: document["lifecycle"].__setitem__(
                "peak_log_bytes", 2 * 1024 * 1024 + 1
            ),
            lambda document: document["lifecycle"].__setitem__(
                "open_handles_delta_after", 1
            ),
            lambda document: document["lifecycle"].__setitem__(
                "temporary_secret_files_after", 1
            ),
        ]
        for mutate in mutations:
            self.reject(mutate)

    def test_manual_baseline_and_redaction_cannot_drift(self) -> None:
        mutations = [
            lambda document: document["manual_baseline"].__setitem__(
                "client_sha256_after", "f" * 64
            ),
            lambda document: document["manual_baseline"].__setitem__(
                "user_config_sha256_after", "f" * 64
            ),
            lambda document: document["manual_baseline"].__setitem__(
                "manual_ssh_after", False
            ),
            lambda document: document["manual_baseline"].__setitem__(
                "uninstall_preserved", False
            ),
            lambda document: document["redaction"].__setitem__("canary_leaks", 1),
            lambda document: document["redaction"].__setitem__(
                "forbidden_fields_absent", False
            ),
            lambda document: document.__setitem__("argv", ["ssh", "private-host"]),
        ]
        for mutate in mutations:
            self.reject(mutate)

    def test_duplicate_and_oversized_manifests_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(
                evidence.NativeOpenSshEvidenceError, "duplicate"
            ):
                evidence.load_manifest(path)

            path.write_bytes(b"x" * (evidence.MAX_MANIFEST_BYTES + 1))
            with self.assertRaisesRegex(
                evidence.NativeOpenSshEvidenceError, "size"
            ):
                evidence.load_manifest(path)

    def test_linked_manifest_is_rejected_when_the_platform_allows_links(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "manifest.json"
            target.write_text(json.dumps(valid_manifest()), encoding="utf-8")
            linked = root / "linked.json"
            try:
                linked.symlink_to(target)
            except OSError:
                self.skipTest("creating a test symlink is unavailable")
            with self.assertRaises(evidence.NativeOpenSshEvidenceError):
                evidence.load_manifest(linked)

    def test_fixed_probe_reports_a_redacted_bounded_state(self) -> None:
        result = evidence.probe_prerequisites()
        self.assertIn(
            result["status"],
            {"ready-for-controlled-native-run", "external-prerequisite"},
        )
        self.assertNotIn("paths", result)
        self.assertIsInstance(result["missing"], list)
        self.assertIsInstance(result["versions"], dict)


if __name__ == "__main__":
    unittest.main()
