#!/usr/bin/env python3
"""Unit tests for pull-request documentation update policy."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


MODULE_PATH = Path(__file__).with_name("check_pr_policy.py")
SPEC = importlib.util.spec_from_file_location("automexia_pr_policy", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load pull-request policy checker")
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)


class PullRequestDocumentationPolicyTests(unittest.TestCase):
    def test_session_launch_and_policy_authorities_are_protected(self) -> None:
        changed = {
            ".github/BRANCH-PROTECTION.md",
            ".github/repository-protection.json",
            "apps/automexia-terminal/src/context/launch_broker.rs",
            "automexia-extension-api/src/lib.rs",
            "automexia-extension-runtime/src/lib.rs",
            "docs/SESSION-LAUNCH-BROKER.md",
            "docs/adr/0012-first-party-ssh-and-session-launch-boundary.md",
            "docs/project/adr/0012-first-party-ssh-and-session-launch-boundary.md",
            "extensions/devops-ssh/src/lib.rs",
            "tests/fixtures/session-launch/d0-d3-contract-v2.json",
            "tests/fixtures/session-launch/d0-d3-contract-v3.json",
            "tests/fixtures/session-launch/d0-d3-contract-v4.json",
            "tests/fixtures/session-launch/d0-d3-contract-v7.json",
            "tests/fixtures/session-launch/native-openssh-evidence-synthetic-v2.json",
            "tests/assurance/release-trust-policy-v1.json",
            "tests/assurance/s1-assurance-policy-v1.json",
            "tests/assurance/performance-ratchet-policy-v1.json",
            "tests/assurance/stable-release-policy-v1.json",
            "tools/ci/check_pr_policy.py",
            "tools/ci/repository_protection.py",
            "tools/ci/test_repository_protection.py",
            "tools/ci/check_session_launch_d0.py",
            "tools/ci/native_openssh_evidence.py",
            "tools/ci/test_native_openssh_evidence.py",
            "tools/ci/release_trust.py",
            "tools/ci/test_release_trust.py",
            "tools/ci/s1_assurance.py",
            "tools/ci/test_s1_assurance.py",
            "tools/ci/performance_assurance.py",
            "tools/ci/test_performance_assurance.py",
            "tools/ci/stable_release.py",
            "tools/ci/test_stable_release.py",
            "tools/xtask/src/main.rs",
        }

        self.assertEqual(POLICY.protected_paths(changed), sorted(changed))

    def test_non_authoritative_roadmap_prose_is_not_a_protected_path(self) -> None:
        changed = {"docs/ROADMAP.md", "docs/user-guide/connection-hub-and-ssh.md"}

        self.assertEqual(POLICY.protected_paths(changed), [])

    def test_independent_approvals_require_the_exact_head_commit(self) -> None:
        old = "a" * 40
        head = "b" * 40
        self.assertEqual(
            POLICY.independent_approval_logins(
                f"Alice|{head},alice|{head.upper()},PR-Author|{head},"
                f"dependabot[bot]|{head},Bob|{head},Carol|{old},"
                f"Mallory\nAPPROVAL_LOGINS=owned|{head},malformed",
                "pr-author",
                head.upper(),
            ),
            {"alice", "bob"},
        )

    def test_stale_or_missing_commit_approvals_do_not_count(self) -> None:
        old = "a" * 40
        head = "b" * 40
        self.assertEqual(
            POLICY.independent_approval_logins(
                f"Alice|{old},Bob|,Carol", "pr-author", head
            ),
            set(),
        )

    def test_ci_collects_the_reviewed_commit_with_each_approval(self) -> None:
        workflow = (
            MODULE_PATH.parents[2] / ".github" / "workflows" / "ci.yml"
        ).read_text(encoding="utf-8")

        self.assertIn("--paginate --slurp |", workflow)
        self.assertIn(
            "python tools/ci/check_pr_policy.py --format-approvals", workflow
        )
        self.assertNotIn("--slurp --jq", workflow)
        self.assertNotIn("jq -r", workflow)

    def test_review_pages_keep_only_each_humans_latest_approval_record(self) -> None:
        old = "a" * 40
        current = "b" * 40
        pages = [
            [
                {
                    "id": 1,
                    "submitted_at": "2026-08-21T10:00:00Z",
                    "state": "APPROVED",
                    "commit_id": old,
                    "user": {"login": "Alice", "type": "User"},
                },
                {
                    "id": 2,
                    "submitted_at": "2026-08-21T11:00:00Z",
                    "state": "DISMISSED",
                    "commit_id": current,
                    "user": {"login": "Alice", "type": "User"},
                },
                {
                    "id": 3,
                    "submitted_at": "2026-08-21T12:00:00Z",
                    "state": "APPROVED",
                    "commit_id": current,
                    "user": {"login": "build-bot", "type": "Bot"},
                },
            ],
            [
                {
                    "id": 4,
                    "submitted_at": "2026-08-21T13:00:00Z",
                    "state": "APPROVED",
                    "commit_id": old,
                    "user": {"login": "Bob", "type": "User"},
                },
                {
                    "id": 5,
                    "submitted_at": "2026-08-21T14:00:00Z",
                    "state": "APPROVED",
                    "commit_id": current,
                    "user": {"login": "Carol", "type": "User"},
                },
                {
                    "id": 6,
                    "submitted_at": "2026-08-21T15:00:00Z",
                    "state": "APPROVED",
                    "commit_id": current,
                    "user": {"login": "Mallory\nINJECTED=1", "type": "User"},
                },
                {
                    "id": 7,
                    "submitted_at": "2026-08-21T16:00:00Z",
                    "state": "APPROVED",
                    "commit_id": current + "\nINJECTED=1",
                    "user": {"login": "David", "type": "User"},
                },
                {"malformed": True},
            ],
        ]

        self.assertEqual(
            POLICY.approval_records_from_review_pages(pages),
            f"Bob|{old},Carol|{current}",
        )

    def test_review_page_count_is_bounded(self) -> None:
        pages = [[{}] * (POLICY.MAX_REVIEW_COUNT + 1)]

        with self.assertRaisesRegex(ValueError, "bounded review count"):
            POLICY.approval_records_from_review_pages(pages)

    def test_source_change_without_documentation_is_rejected(self) -> None:
        self.assertEqual(
            POLICY.missing_documentation_for({"apps/automexia-terminal/src/main.rs"}),
            ["apps/automexia-terminal/src/main.rs"],
        )

    def test_documentation_in_same_change_satisfies_policy(self) -> None:
        self.assertEqual(
            POLICY.missing_documentation_for(
                {
                    "apps/automexia-terminal/src/main.rs",
                    "docs/CLI-REFERENCE.md",
                    "changes/feature.md",
                }
            ),
            [],
        )

    def test_test_workflow_and_configuration_changes_require_docs(self) -> None:
        changed = {
            ".github/workflows/ci.yml",
            "Cargo.lock",
            "tests/fixtures/example.json",
            "tools/ci/check_example.py",
        }
        self.assertEqual(
            POLICY.missing_documentation_for(changed),
            sorted(changed),
        )

    def test_assets_and_packaging_changes_require_docs(self) -> None:
        changed = {"assets/brand/icon.svg", "packaging/windows/main.wxs"}
        self.assertEqual(
            POLICY.missing_documentation_for(changed),
            sorted(changed),
        )

    def test_changelog_and_documentation_only_changes_do_not_require_more_docs(
        self,
    ) -> None:
        self.assertEqual(
            POLICY.missing_documentation_for(
                {"changes/docs.md", "docs/DOCUMENTATION.md"}
            ),
            [],
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
