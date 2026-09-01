#!/usr/bin/env python3
"""Regression and mutation tests for hosted CI and repository protection."""

from __future__ import annotations

import copy
import datetime as dt
import importlib.util
from pathlib import Path
import subprocess
import sys
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
        # Each mutation starts from the repository policy as shipped; deep copies
        # below ensure a rejected weakening never leaks into another test.
        self.policy = PROTECTION.load_policy()

    def test_current_policy_and_workflows_satisfy_the_contract(self) -> None:
        counts = PROTECTION.validate_repository(self.policy)

        self.assertEqual(counts["rulesets"], 0)
        self.assertEqual(counts["required_checks"], 3)
        self.assertEqual(counts["action_patterns"], 4)
        self.assertGreaterEqual(counts["codeowners"], 1)

    def test_duplicate_json_keys_fail_closed(self) -> None:
        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "duplicate key"):
            PROTECTION.parse_json('{"schema": 1, "schema": 2}')

    def test_codeowners_requires_a_valid_repository_wide_owner(self) -> None:
        # A narrow path rule is not a fallback owner: release-sensitive files can
        # appear anywhere, so the parser must retain an explicit repository-wide rule.
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
        altered["schema"] = 3

        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "policy identity"):
            PROTECTION.validate_policy(altered)

    def test_paid_server_controls_cannot_be_required(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["server_side_paid_controls_required"] = True

        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "must not require"):
            PROTECTION.validate_policy(altered)

    def test_release_policy_cannot_drop_exact_head_controls(self) -> None:
        for key, value in (
            ("same_repository_only", False),
            ("must_equal_current_main", False),
            ("default_minimum_human_approvals", 0),
        ):
            altered = copy.deepcopy(self.policy)
            altered["release"][key] = value
            with self.subTest(key=key), self.assertRaisesRegex(
                PROTECTION.ProtectionPolicyError, "GitHub-Free release"
            ):
                PROTECTION.validate_policy(altered)

    def test_required_checks_cannot_drift_from_workflow_job_names(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["ordinary_ci"]["required_checks"].pop()

        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "required checks"):
            PROTECTION.validate_policy(altered)

    def test_ordinary_ci_cannot_restore_non_free_runner(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["ordinary_ci"]["hosted_os"] = ["windows-2025"]

        with self.assertRaisesRegex(
            PROTECTION.ProtectionPolicyError, "GitHub-Free Ubuntu runner"
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

    def test_linux_early_access_is_registered_without_claiming_default_branch_evidence(self) -> None:
        self.assertEqual(
            PROTECTION.EXPECTED_WORKFLOWS["linux-early-access.yml"],
            "Linux Early Access release",
        )
        self.assertNotIn(
            "linux-early-access.yml",
            PROTECTION.EXPECTED_DEFAULT_BRANCH_EVIDENCE_WORKFLOWS,
        )

    def test_action_policy_requires_sha_pinning_and_an_exact_allowlist(self) -> None:
        for field, value in (
            ("full_length_sha_required", False),
            ("mode", "all-actions"),
        ):
            altered = copy.deepcopy(self.policy)
            altered["actions_policy"][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(
                PROTECTION.ProtectionPolicyError, "selected Action policy"
            ):
                PROTECTION.validate_policy(altered)

    def test_action_allowlist_cannot_drop_a_used_third_party_action(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["actions_policy"]["third_party_patterns"].pop()

        with self.assertRaisesRegex(PROTECTION.ProtectionPolicyError, "selected Action policy"):
            PROTECTION.validate_policy(altered)

    def test_excluded_paid_features_cannot_drift(self) -> None:
        altered = copy.deepcopy(self.policy)
        altered["paid_github_features_not_used"].pop()

        with self.assertRaisesRegex(
            PROTECTION.ProtectionPolicyError, "excluded feature inventory"
        ):
            PROTECTION.validate_policy(altered)

    def test_private_free_plan_error_is_external_not_compliant(self) -> None:
        result = PROTECTION.classify_api_error(
            403,
            "Upgrade to GitHub Pro or make this repository public to enable this feature.",
        )

        self.assertEqual(result, "external-plan")

    def test_free_private_audit_remains_external_and_fail_closed(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(MODULE_PATH), "audit", "--json"],
            cwd=MODULE_PATH.parents[2],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        self.assertEqual(completed.returncode, 2)
        self.assertIn("github-free-manual-governance", completed.stdout)

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
        payload = self.policy["actions_policy"]

        self.assertIsInstance(payload["third_party_patterns"], list)
        self.assertEqual(
            payload["third_party_patterns"], sorted(payload["third_party_patterns"])
        )

    def test_hosted_success_must_belong_to_the_protected_head(self) -> None:
        current = "a" * 40

        self.assertTrue(PROTECTION.hosted_run_matches_head({"head_sha": current}, current))
        self.assertFalse(
            PROTECTION.hosted_run_matches_head({"head_sha": "b" * 40}, current)
        )
        self.assertFalse(PROTECTION.hosted_run_matches_head({}, current))

    def test_remote_findings_distinguish_drift_and_external_prerequisites(self) -> None:
        self.assertEqual(
            PROTECTION.classify_api_error(
                403,
                "Upgrade to GitHub Pro or make this repository public to enable this feature.",
            ),
            "external-plan",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
