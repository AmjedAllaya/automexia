#!/usr/bin/env python3
"""Regression and mutation tests for hosted CI and repository protection."""

from __future__ import annotations

import copy
import datetime as dt
import importlib.util
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("repository_protection.py")
SPEC = importlib.util.spec_from_file_location("automexia_repository_protection", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load repository protection checker")
PROTECTION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROTECTION)


class RepositoryProtectionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = PROTECTION.load_policy()

    def test_current_policy_and_workflows_satisfy_the_contract(self) -> None:
        counts = PROTECTION.validate_repository(self.policy)

        self.assertEqual(counts["rulesets"], 2)
        self.assertGreaterEqual(counts["required_checks"], 14)
        self.assertGreaterEqual(counts["action_patterns"], 6)
        self.assertGreaterEqual(counts["codeowners"], 1)

    def test_duplicate_json_keys_fail_closed(self) -> None:
        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "duplicate key"):
            PROTECTION.parse_json('{"schema": 1, "schema": 2}')

    def test_codeowners_requires_a_valid_repository_wide_owner(self) -> None:
        self.assertEqual(
            PROTECTION.parse_codeowners("# fallback\n* @AmjedAllaya\n"),
            ["@AmjedAllaya"],
        )
        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "repository-wide"):
            PROTECTION.parse_codeowners("docs/ @AmjedAllaya\n")
        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "invalid owner"):
            PROTECTION.parse_codeowners("* not-an-owner\n")

    def test_codeowners_rejects_non_utf8_input(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "CODEOWNERS"
            path.write_bytes(b"* @owner\n\xff")

            with self.assertRaisesRegex(
                PROTECTION.ProtectionPolicyError, "must be UTF-8"
            ):
                PROTECTION.validate_codeowners(path)

    def test_repository_identity_cannot_drift(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["repository"] = "someone/else"

        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "repository identity"):
            PROTECTION.validate_policy(altered)

    def test_main_ruleset_cannot_gain_a_bypass(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["rulesets"][0]["bypass_actors"] = [
            {"actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always"}
        ]

        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "bypass"):
            PROTECTION.validate_policy(altered)

    def test_main_ruleset_cannot_drop_pull_requests_or_signatures(self) -> None:
        for rule_type in ("pull_request", "required_signatures"):
            altered = copy.deepcopy(self.policy)
            altered["rulesets"][0]["rules"] = [
                rule for rule in altered["rulesets"][0]["rules"]
                if rule["type"] != rule_type
            ]
            with self.subTest(rule=rule_type), self.assertRaisesRegex(
                PROTECTION.ProtectionPolicyError, rule_type
            ):
                PROTECTION.validate_policy(altered)

    def test_required_checks_cannot_drift_from_workflow_job_names(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["hosted_ci"]["required_pull_request_checks"].remove(
            "Native windows-latest"
        )

        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "required checks"):
            PROTECTION.validate_policy(altered)

    def test_default_branch_evidence_workflow_set_cannot_drift(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["hosted_ci"]["default_branch_evidence_workflows"] = ["ci.yml"]

        with self.assertRaisesRegex(
            PROTECTION.ProtectionPolicyError, "default-branch evidence workflows"
        ):
            PROTECTION.validate_policy(altered)

    def test_s1_controlled_workflow_is_registered_without_claiming_default_branch_evidence(self) -> None:
        self.assertEqual(
            PROTECTION.EXPECTED_WORKFLOWS["s1-assurance.yml"],
            "S1 controlled assurance",
        )
        self.assertNotIn("s1-assurance.yml", PROTECTION.EXPECTED_DEFAULT_BRANCH_EVIDENCE_WORKFLOWS)

    def test_f5_controlled_workflow_is_registered_without_claiming_default_branch_evidence(self) -> None:
        self.assertEqual(
            PROTECTION.EXPECTED_WORKFLOWS["f5-openssh-assurance.yml"],
            "F5 controlled native OpenSSH assurance",
        )
        self.assertNotIn(
            "f5-openssh-assurance.yml",
            PROTECTION.EXPECTED_DEFAULT_BRANCH_EVIDENCE_WORKFLOWS,
        )

    def test_s2_activation_workflow_is_registered_without_claiming_default_branch_evidence(self) -> None:
        self.assertEqual(
            PROTECTION.EXPECTED_WORKFLOWS["s2-assurance.yml"],
            "S2 controlled activation",
        )
        self.assertNotIn(
            "s2-assurance.yml", PROTECTION.EXPECTED_DEFAULT_BRANCH_EVIDENCE_WORKFLOWS
        )

    def test_action_policy_requires_sha_pinning_and_an_exact_allowlist(self) -> None:
        for field, value in (
            ("sha_pinning_required", False),
            ("allowed_actions", "all"),
        ):
            altered = copy.deepcopy(self.policy)
            altered["actions"]["permissions"][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(
                PROTECTION.ProtectionPolicyError, "Actions permissions"
            ):
                PROTECTION.validate_policy(altered)

    def test_workflow_tokens_cannot_approve_pull_requests(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["actions"]["workflow_permissions"][
            "can_approve_pull_request_reviews"
        ] = True

        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "workflow token"):
            PROTECTION.validate_policy(altered)

    def test_security_features_remain_fail_closed_when_available(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["security"]["secret_scanning"] = "optional"

        with self.assertRaisesRegex(
            PROTECTION.ProtectionPolicyError, "security feature policy|secret scanning"
        ):
            PROTECTION.validate_policy(altered)

    def test_private_free_plan_error_is_external_not_compliant(self) -> None:
        result = PROTECTION.classify_api_error(
            403,
            "Upgrade to GitHub Pro or make this repository public to enable this feature.",
        )

        self.assertEqual(result, "external-plan")

    def test_zero_step_payment_rejection_is_not_a_ci_failure(self) -> None:
        result = PROTECTION.classify_hosted_job(
            {
                "conclusion": "failure",
                "runner_id": 0,
                "steps": [],
                "annotation": (
                    "The job was not started because recent account payments have failed "
                    "or your spending limit needs to be increased."
                ),
            }
        )

        self.assertEqual(result, "external-billing")

    def test_executed_failure_remains_a_real_failure(self) -> None:
        result = PROTECTION.classify_hosted_job(
            {
                "conclusion": "failure",
                "runner_id": 42,
                "steps": [{"name": "Checkout", "conclusion": "success"}],
                "annotation": "",
            }
        )

        self.assertEqual(result, "fail")

    def test_skipped_only_or_empty_hosted_job_sets_do_not_pass(self) -> None:
        self.assertFalse(PROTECTION.hosted_jobs_pass([]))
        self.assertFalse(PROTECTION.hosted_jobs_pass(["skip"]))
        self.assertTrue(PROTECTION.hosted_jobs_pass(["pass", "pass"]))

    def test_stale_or_future_success_cannot_satisfy_hosted_recency(self) -> None:
        now = dt.datetime(2026, 8, 24, 12, tzinfo=dt.timezone.utc)

        self.assertTrue(
            PROTECTION.hosted_run_is_recent(
                {"updated_at": "2026-08-23T12:00:00Z"}, 7, now=now
            )
        )
        self.assertFalse(
            PROTECTION.hosted_run_is_recent(
                {"updated_at": "2026-08-16T11:59:59Z"}, 7, now=now
            )
        )
        self.assertFalse(
            PROTECTION.hosted_run_is_recent(
                {"updated_at": "2026-08-25T12:00:00Z"}, 7, now=now
            )
        )

    def test_selected_actions_apply_payload_remains_an_array(self) -> None:
        payload = PROTECTION.selected_actions_apply_payload(self.policy)

        self.assertIsInstance(payload["patterns_allowed"], list)
        self.assertEqual(payload["patterns_allowed"], sorted(payload["patterns_allowed"]))

    def test_hosted_success_must_belong_to_the_protected_head(self) -> None:
        current = "a" * 40

        self.assertTrue(PROTECTION.hosted_run_matches_head({"head_sha": current}, current))
        self.assertFalse(
            PROTECTION.hosted_run_matches_head({"head_sha": "b" * 40}, current)
        )
        self.assertFalse(PROTECTION.hosted_run_matches_head({}, current))

    def test_remote_findings_distinguish_drift_and_external_prerequisites(self) -> None:
        findings = PROTECTION.evaluate_remote_snapshot(
            self.policy,
            PROTECTION.synthetic_compliant_snapshot(self.policy)
            | {
                "rulesets_availability": "external-plan",
                "human_reviewer_count": 1,
                "hosted_ci": "external-billing",
            },
        )

        statuses = {finding.identifier: finding.status for finding in findings}
        self.assertEqual(statuses["rulesets"], "external")
        self.assertEqual(statuses["reviewer-capacity"], "external")
        self.assertEqual(statuses["hosted-ci"], "external")
        self.assertNotIn("pass", {statuses[key] for key in statuses if key in {
            "rulesets", "reviewer-capacity", "hosted-ci"
        }})


if __name__ == "__main__":
    unittest.main(verbosity=2)
