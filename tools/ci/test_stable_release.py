"""Regression tests for the stable-release source and repository gate."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import stable_release


class StableReleaseTests(unittest.TestCase):
    def git(self, root: Path, *args: str) -> str:
        completed = subprocess.run(
            ["git", *args],
            cwd=root,
            check=True,
            capture_output=True,
            text=True,
            timeout=15,
        )
        return completed.stdout.strip()

    def fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path, str]:
        temporary = tempfile.TemporaryDirectory()
        outer = Path(temporary.name)
        remote = outer / "origin.git"
        work = outer / "work"
        self.git(outer, "init", "--bare", str(remote))
        self.git(outer, "clone", str(remote), str(work))
        self.git(work, "config", "user.name", "Release Tester")
        self.git(work, "config", "user.email", "release@example.invalid")
        self.git(work, "checkout", "-b", "main")
        (work / "source.txt").write_text("base\n", encoding="utf-8")
        self.git(work, "add", "source.txt")
        self.git(
            work,
            "commit",
            "-m",
            "chore: establish fork base",
            "-m",
            "Signed-off-by: Release Tester <release@example.invalid>",
        )
        base = self.git(work, "rev-parse", "HEAD")
        self.git(work, "tag", "-a", "fork-base", "-m", "audited fork base")
        (work / "source.txt").write_text("release\n", encoding="utf-8")
        self.git(work, "add", "source.txt")
        self.git(
            work,
            "commit",
            "-m",
            "feat: prepare stable release",
            "-m",
            "Signed-off-by: Release Tester <release@example.invalid>",
        )
        self.git(work, "tag", "-a", "v1.2.3", "-m", "stable release")
        self.git(work, "push", "origin", "main", "fork-base", "v1.2.3")
        return temporary, work, base

    def fixture_policy(self, base: str) -> dict[str, object]:
        policy = stable_release.load_policy()
        policy["repository"] = "fixture/repository"
        policy["default_branch"] = "main"
        policy["fork_provenance"] = {
            "tag": "fork-base",
            "commit": base,
            "require_annotated_tag": True,
            "require_remote_tag": True,
        }
        policy["release_source"]["tag_pattern"] = r"^v1\.2\.3$"
        return policy

    def test_current_policy_and_workflow_are_complete(self) -> None:
        policy = stable_release.load_policy()
        stable_release.validate_policy(policy)
        stable_release.validate_release_workflow(policy)

    def test_duplicate_policy_keys_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "policy.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(stable_release.ReleaseError, "duplicate"):
                stable_release.load_policy(path)

    def test_failed_git_command_returns_redacted_release_error(self) -> None:
        isolated_parent = Path(stable_release.ROOT.anchor) if os.name == "nt" else None
        with tempfile.TemporaryDirectory(prefix="automexia-non-repository-", dir=isolated_parent) as temporary:
            with self.assertRaisesRegex(
                stable_release.ReleaseError, 'git rev-parse operation failed'
            ):
                stable_release._run_git(Path(temporary), 'rev-parse', 'HEAD')

    def test_external_prerequisite_identity_cannot_be_substituted(self) -> None:
        policy = stable_release.load_policy()
        policy["external_prerequisites"][0]["id"] = "substituted-blocker"
        with self.assertRaisesRegex(stable_release.ReleaseError, "identities drifted"):
            stable_release.validate_policy(policy)

    def test_policy_rejects_unknown_fields_and_weakened_tag_grammar(self) -> None:
        policy = stable_release.load_policy()
        policy["release_source"]["allow_lightweight_tag"] = True
        with self.assertRaisesRegex(stable_release.ReleaseError, "keys drifted"):
            stable_release.validate_policy(policy)

        policy = stable_release.load_policy()
        policy["release_source"]["tag_pattern"] = ".*"
        with self.assertRaisesRegex(stable_release.ReleaseError, "tag pattern drifted"):
            stable_release.validate_policy(policy)

        policy = stable_release.load_policy()
        policy["hosted_repository"]["github_free_private_manual_governance"] = False
        with self.assertRaisesRegex(
            stable_release.ReleaseError, "github_free_private_manual_governance"
        ):
            stable_release.validate_policy(policy)

    def test_external_prerequisite_owner_and_evidence_cannot_be_redirected(self) -> None:
        policy = stable_release.load_policy()
        policy["external_prerequisites"][0]["owner"] = "different-owner"
        with self.assertRaisesRegex(stable_release.ReleaseError, "contract drifted"):
            stable_release.validate_policy(policy)

        policy = stable_release.load_policy()
        policy["external_prerequisites"][0]["evidence"] = "Cargo.toml"
        with self.assertRaisesRegex(stable_release.ReleaseError, "contract drifted"):
            stable_release.validate_policy(policy)

    def test_dco_history_is_read_once_with_a_bounded_batch(self) -> None:
        first = "1" * 40
        second = "2" * 40
        output = (
            f"{first}\0release@example.invalid\0"
            "feat: first\n\nSigned-off-by: Release Tester <release@example.invalid>\0"
            f"{second}\0release@example.invalid\0"
            "fix: second\n\nSigned-off-by: Release Tester <release@example.invalid>\0"
        )
        with mock.patch.object(stable_release, "_run_git", return_value=output) as run:
            stable_release._validate_dco(Path("."), "0" * 40, second)
        run.assert_called_once_with(
            Path("."),
            "log",
            "-z",
            "--reverse",
            "--format=%H%x00%ae%x00%B",
            f"{'0' * 40}..{second}",
        )
    def test_valid_annotated_exact_main_release_passes(self) -> None:
        temporary, work, base = self.fixture()
        self.addCleanup(temporary.cleanup)
        expected = self.git(work, "rev-parse", "HEAD")
        stable_release.validate_source(
            self.fixture_policy(base),
            work,
            expected_commit=expected,
            release_tag="v1.2.3",
            remote="origin",
        )

    def test_lightweight_release_tag_is_rejected(self) -> None:
        temporary, work, base = self.fixture()
        self.addCleanup(temporary.cleanup)
        self.git(work, "tag", "-d", "v1.2.3")
        self.git(work, "tag", "v1.2.3")
        expected = self.git(work, "rev-parse", "HEAD")
        with self.assertRaisesRegex(stable_release.ReleaseError, "annotated"):
            stable_release.validate_source(
                self.fixture_policy(base), work, expected, "v1.2.3", "origin"
            )

    def test_release_must_equal_remote_default_branch_head(self) -> None:
        temporary, work, base = self.fixture()
        self.addCleanup(temporary.cleanup)
        previous = self.git(work, "rev-parse", "HEAD")
        (work / "later.txt").write_text("later\n", encoding="utf-8")
        self.git(work, "add", "later.txt")
        self.git(
            work,
            "commit",
            "-m",
            "chore: advance main",
            "-m",
            "Signed-off-by: Release Tester <release@example.invalid>",
        )
        self.git(work, "push", "origin", "main")
        self.git(work, "checkout", "--detach", previous)
        with self.assertRaisesRegex(stable_release.ReleaseError, "default branch"):
            stable_release.validate_source(
                self.fixture_policy(base), work, previous, "v1.2.3", "origin"
            )

    def test_missing_remote_fork_tag_is_rejected(self) -> None:
        temporary, work, base = self.fixture()
        self.addCleanup(temporary.cleanup)
        self.git(work, "push", "origin", ":refs/tags/fork-base")
        expected = self.git(work, "rev-parse", "HEAD")
        with self.assertRaisesRegex(stable_release.ReleaseError, "remote fork"):
            stable_release.validate_source(
                self.fixture_policy(base), work, expected, "v1.2.3", "origin"
            )

    def test_missing_remote_release_tag_is_rejected(self) -> None:
        temporary, work, base = self.fixture()
        self.addCleanup(temporary.cleanup)
        self.git(work, "push", "origin", ":refs/tags/v1.2.3")
        expected = self.git(work, "rev-parse", "HEAD")
        with self.assertRaisesRegex(stable_release.ReleaseError, "remote release tag"):
            stable_release.validate_source(
                self.fixture_policy(base), work, expected, "v1.2.3", "origin"
            )

    def test_shallow_checkout_is_rejected_before_provenance_walk(self) -> None:
        temporary, work, base = self.fixture()
        self.addCleanup(temporary.cleanup)
        shallow = work.parent / "shallow"
        self.git(
            work.parent,
            "clone",
            "--depth",
            "1",
            "--branch",
            "main",
            (work.parent / "origin.git").as_uri(),
            str(shallow),
        )
        expected = self.git(shallow, "rev-parse", "HEAD")
        with self.assertRaisesRegex(stable_release.ReleaseError, "complete Git history"):
            stable_release.validate_source(
                self.fixture_policy(base), shallow, expected, "v1.2.3", "origin"
            )
    def test_missing_dco_trailer_is_rejected(self) -> None:
        temporary, work, base = self.fixture()
        self.addCleanup(temporary.cleanup)
        self.git(work, "tag", "-d", "v1.2.3")
        (work / "unsigned.txt").write_text("unsigned\n", encoding="utf-8")
        self.git(work, "add", "unsigned.txt")
        self.git(work, "commit", "-m", "test: omit signoff")
        self.git(work, "tag", "-a", "v1.2.3", "-m", "stable release")
        self.git(work, "push", "--force", "origin", "main", "v1.2.3")
        expected = self.git(work, "rev-parse", "HEAD")
        with self.assertRaisesRegex(stable_release.ReleaseError, "DCO"):
            stable_release.validate_source(
                self.fixture_policy(base), work, expected, "v1.2.3", "origin"
            )

    def test_merge_commit_after_fork_is_rejected(self) -> None:
        temporary, work, base = self.fixture()
        self.addCleanup(temporary.cleanup)
        self.git(work, "tag", "-d", "v1.2.3")
        self.git(work, "checkout", "-b", "merge-side", base)
        (work / "side.txt").write_text("side\n", encoding="utf-8")
        self.git(work, "add", "side.txt")
        self.git(
            work,
            "commit",
            "-m",
            "test: create side",
            "-m",
            "Signed-off-by: Release Tester <release@example.invalid>",
        )
        self.git(work, "checkout", "main")
        self.git(
            work,
            "merge",
            "--no-ff",
            "merge-side",
            "-m",
            "test: merge side",
            "-m",
            "Signed-off-by: Release Tester <release@example.invalid>",
        )
        self.git(work, "tag", "-a", "v1.2.3", "-m", "stable release")
        self.git(work, "push", "--force", "origin", "main", "v1.2.3")
        expected = self.git(work, "rev-parse", "HEAD")
        with self.assertRaisesRegex(stable_release.ReleaseError, "not linear"):
            stable_release.validate_source(
                self.fixture_policy(base), work, expected, "v1.2.3", "origin"
            )
    def test_dirty_tracked_source_is_rejected(self) -> None:
        temporary, work, base = self.fixture()
        self.addCleanup(temporary.cleanup)
        (work / "source.txt").write_text("dirty\n", encoding="utf-8")
        expected = self.git(work, "rev-parse", "HEAD")
        with self.assertRaisesRegex(stable_release.ReleaseError, "tracked source"):
            stable_release.validate_source(
                self.fixture_policy(base), work, expected, "v1.2.3", "origin"
            )

    def test_workflow_cannot_drop_authenticated_repository_audit(self) -> None:
        policy = stable_release.load_policy()
        workflow = stable_release.RELEASE_WORKFLOW.read_text(encoding="utf-8")
        workflow = workflow.replace(
            "python tools/ci/repository_protection.py audit --json", "true"
        )
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "release.yml"
            path.write_text(workflow, encoding="utf-8")
            with self.assertRaisesRegex(stable_release.ReleaseError, "repository audit"):
                stable_release.validate_release_workflow(policy, path)

    def test_workflow_cannot_restore_private_environment_dependency(self) -> None:
        policy = stable_release.load_policy()
        workflow = stable_release.RELEASE_WORKFLOW.read_text(encoding="utf-8")
        workflow = workflow.replace(
            "  preflight:\n",
            "  preflight:\n    environment: stable-release\n",
            1,
        )
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "release.yml"
            path.write_text(workflow, encoding="utf-8")
            with self.assertRaisesRegex(
                stable_release.ReleaseError, "private GitHub environments"
            ):
                stable_release.validate_release_workflow(policy, path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
