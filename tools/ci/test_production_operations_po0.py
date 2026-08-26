#!/usr/bin/env python3
"""Mutation tests for the proposed, non-activating PO0 contract."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import unittest


MODULE_PATH = Path(__file__).with_name("check_production_operations_po0.py")
SPEC = importlib.util.spec_from_file_location(
    "automexia_production_operations_po0", MODULE_PATH
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load Production Operations PO0 checker")
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class ProductionOperationsPo0Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = (CHECKER.ROOT / CHECKER.CONTRACT_PATH).read_text(
            encoding="utf-8"
        )
        cls.document = CHECKER.parse_contract(cls.text)

    def assert_semantically_rejected(self, mutation) -> None:
        document = copy.deepcopy(self.document)
        mutation(document)
        with self.assertRaises(CHECKER.ProductionOperationsContractError):
            CHECKER.validate_contract(document, check_digest=False)

    def test_repository_contract_passes(self) -> None:
        counts = CHECKER.validate_repository()
        self.assertEqual(counts["record_kinds"], 17)
        self.assertEqual(counts["record_payloads"], 17)
        self.assertEqual(counts["hard_gates"], 9)
        self.assertEqual(counts["provider_profiles"], 7)
        self.assertEqual(counts["action_profiles"], 7)
        self.assertEqual(counts["actions"], 37)
        self.assertEqual(counts["limits"], 38)
        self.assertEqual(counts["traceability"], 8)
        self.assertEqual(counts["external_gates"], 10)
        self.assertGreater(counts["source_files_checked"], 100)

    def test_duplicate_json_key_is_rejected(self) -> None:
        duplicate = self.text.replace(
            '"schema": 1,', '"schema": 1,\n  "schema": 1,', 1
        )
        with self.assertRaisesRegex(
            CHECKER.ProductionOperationsContractError, "duplicate key"
        ):
            CHECKER.parse_contract(duplicate)

    def test_identity_status_and_digest_are_frozen(self) -> None:
        self.assert_semantically_rejected(
            lambda value: value.update(status="implemented")
        )
        document = copy.deepcopy(self.document)
        document["traceability"][0]["requirement"] += "-changed"
        with self.assertRaisesRegex(
            CHECKER.ProductionOperationsContractError, "digest"
        ):
            CHECKER.validate_contract(document)

    def test_authority_cannot_activate(self) -> None:
        for key in (
            "accepted",
            "runtime_activation",
            "provider_requests",
            "watchers",
            "completion_source",
            "settings",
            "shortcuts",
            "journal",
            "managed_execution",
            "managed_sessions",
            "organization_packs",
            "local_model",
        ):
            with self.subTest(key=key):
                self.assert_semantically_rejected(
                    lambda value, key=key: value["authority"].update({key: True})
                )
        self.assert_semantically_rejected(
            lambda value: value["authority"].update(new_dependencies="kube")
        )
        self.assert_semantically_rejected(
            lambda value: value["authority"].update(fallback="po-only")
        )

    def test_owner_and_format_boundaries_cannot_weaken(self) -> None:
        self.assert_semantically_rejected(
            lambda value: value["ownership"]["forbidden_duplicate_owners"].remove(
                "process-runner"
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["formats"].update(duplicate_keys="accept")
        )
        self.assert_semantically_rejected(
            lambda value: value["formats"].update(unknown_major="ignore")
        )
        self.assert_semantically_rejected(
            lambda value: value["formats"].update(
                planning_fixture_is_runtime_input=True
            )
        )

    def test_evidence_uncertainty_and_freshness_are_frozen(self) -> None:
        self.assert_semantically_rejected(
            lambda value: value["common_envelope"].remove("expires-at")
        )
        self.assert_semantically_rejected(
            lambda value: value["environment_classification_precedence"].pop()
        )
        self.assert_semantically_rejected(
            lambda value: value["evidence_quality"].update(
                opaque_average_score=True
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["evidence_quality"].update(
                correlation_is_causation=True
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["freshness_seconds"].update(
                target_health_production=60
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["freshness_seconds"].update(one_use_grant=300)
        )

    def test_every_record_payload_is_frozen(self) -> None:
        for kind in CHECKER.EXPECTED_RECORD_PAYLOADS:
            with self.subTest(kind=kind):
                self.assert_semantically_rejected(
                    lambda value, kind=kind: value["record_payloads"][kind].pop()
                )
        self.assert_semantically_rejected(
            lambda value: value["record_payloads"]["candidate-action"].remove(
                "hard-gate-results"
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["record_payloads"]["managed-action-grant"].remove(
                "consumption-state"
            )
        )

    def test_gates_ranking_and_rollout_refusals_are_frozen(self) -> None:
        self.assert_semantically_rejected(
            lambda value: value["hard_gates"].pop()
        )
        self.assert_semantically_rejected(
            lambda value: value["ranking"].update(automatic_execution=True)
        )
        self.assert_semantically_rejected(
            lambda value: value["ranking"].update(
                read_only_precedes_mutation_when_otherwise_equal=False
            )
        )
        for marker in (
            "image-pull-restart-refused",
            "pending-unschedulable-restart-refused",
            "crashloop-is-symptom-not-root-cause",
            "stale-conflicting-or-owner-unknown-refuses-mutation",
        ):
            with self.subTest(marker=marker):
                self.assert_semantically_rejected(
                    lambda value, marker=marker: value[
                        "kubernetes_rollout_rules"
                    ].remove(marker)
                )

    def test_policy_and_operation_lifecycle_fail_closed(self) -> None:
        self.assert_semantically_rejected(
            lambda value: value["policy"].update(deny_wins=False)
        )
        self.assert_semantically_rejected(
            lambda value: value["policy"].update(
                unknown_blocks_production_mutation=False
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["policy"].update(
                lower_level_may_override_deny=True
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["operation_invariants"].remove(
                "process-exit-is-not-success"
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["operation_invariants"].remove(
                "no-second-mutation-chain"
            )
        )

    def test_provider_profiles_preserve_explicit_context_and_advisory_language(self) -> None:
        mutations = (
            ("kubernetes", "bounded-list-watch-resource-version-relist-on-410"),
            ("kubernetes", "no-kubectl-external-diff-or-show-secrets"),
            ("aws", "explicit-named-profile"),
            ("aws", "iam-simulation-advisory-only"),
            ("azure", "never-az-account-set"),
            ("gcp", "never-activate-global-configuration"),
            ("gitops_iac", "reconcile-and-sync-are-mutations"),
            (
                "gitops_iac",
                "terraform-opentofu-raw-plan-and-state-never-ingested-or-persisted",
            ),
        )
        for profile, marker in mutations:
            with self.subTest(profile=profile, marker=marker):
                self.assert_semantically_rejected(
                    lambda value, profile=profile, marker=marker: value[
                        "provider_profiles"
                    ][profile].remove(marker)
                )

    def test_action_profiles_preserve_phase_effect_and_authority(self) -> None:
        for profile in CHECKER.EXPECTED_ACTION_PROFILES:
            with self.subTest(profile=profile):
                self.assert_semantically_rejected(
                    lambda value, profile=profile: value["action_profiles"][
                        profile
                    ].pop()
                )
        self.assert_semantically_rejected(
            lambda value: value["action_profiles"]["kubernetes"][7].update(
                effect="read-only"
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["action_profiles"]["gitops_iac"][3].update(
                phase="PO4"
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["action_profiles"]["openshift"][0].update(
                authority="ambient-context"
            )
        )

    def test_collection_cannot_enter_hot_paths_or_capture_raw_data(self) -> None:
        for key in (
            "typing_io",
            "startup_io",
            "watch_bookmark_is_object_freshness",
            "raw_logs_or_metric_series_cached",
            "raw_terraform_plan_or_state_ingested",
        ):
            with self.subTest(key=key):
                self.assert_semantically_rejected(
                    lambda value, key=key: value["collection_strategy"].update(
                        {key: True}
                    )
                )
        self.assert_semantically_rejected(
            lambda value: value["collection_strategy"].update(
                sustained_kubernetes="hand-written-watch-client"
            )
        )

    def test_planned_settings_actions_and_journal_remain_disabled(self) -> None:
        for key in (
            "enabled_default",
            "suggestions_default",
            "live_logs_default",
            "managed_actions_default",
            "port_forward_default",
            "active_probe_default",
            "debug_workload_default",
            "allow_detach_default",
            "current_config_schema_contains_these_keys",
        ):
            with self.subTest(key=key):
                self.assert_semantically_rejected(
                    lambda value, key=key: value[
                        "planned_configuration"
                    ].update({key: True})
                )
        self.assert_semantically_rejected(
            lambda value: value["planned_actions"].update(
                default_shortcut="ctrl+enter"
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["planned_actions"].update(
                selection_can_execute_managed_mutation=True
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["journal"].update(persistent_default=True)
        )
        self.assert_semantically_rejected(
            lambda value: value["journal"]["forbidden"].remove(
                "credentials-tokens-certificates-or-secrets"
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["journal"]["forbidden"].remove(
                "raw-logs-or-metrics"
            )
        )

    def test_limits_cannot_expand_or_remove_zero_io(self) -> None:
        self.assert_semantically_rejected(
            lambda value: value["limits"].update(
                typing_provider_filesystem_process_or_network_operations=1
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["limits"].update(active_routes_global=256)
        )
        self.assert_semantically_rejected(
            lambda value: value["limits"].update(
                persistent_journal_bytes=268435456
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["limits"].update(mutation_retries=1)
        )

    def test_accessibility_traceability_and_external_gates_cannot_erode(self) -> None:
        self.assert_semantically_rejected(
            lambda value: value["accessibility"].update(
                focus_and_selection_separate=False
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["accessibility"].update(
                color_or_icon_only_meaning=True
            )
        )
        self.assert_semantically_rejected(
            lambda value: value["traceability"].pop()
        )
        self.assert_semantically_rejected(
            lambda value: value.update(external_gates=[])
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
