#!/usr/bin/env python3
"""Regression tests for legacy pack integrity and the private-doc boundary."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest import mock


MODULE_PATH = Path(__file__).with_name("check_repository_aligned_docs.py")
SPEC = importlib.util.spec_from_file_location(
    "automexia_repository_aligned_docs", MODULE_PATH
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load repository-aligned documentation checker")
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


BASELINE = "20ff7928ea2d1eac5d26f13c62d0cda9b86bc078"


class RepositoryAlignedDocumentationTests(unittest.TestCase):
    def create_pack(self, root: Path, files: dict[str, str]) -> Path:
        pack = root / CHECKER.PACK_PATH
        pack.mkdir(parents=True)
        for relative, content in files.items():
            target = pack / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(content.encode("utf-8"))

        entries = []
        for relative in sorted(files):
            payload = (pack / relative).read_bytes()
            entries.append(
                {
                    "path": relative,
                    "bytes": len(payload),
                    "sha256": hashlib.sha256(payload).hexdigest(),
                    "historical": relative.startswith("historical/"),
                }
            )
        manifest = {
            "schema": 1,
            "generated_at": "2026-08-23",
            "pack_status": "repository-aligned research/proposal pack",
            "project_authority": False,
            "source_audit_head": BASELINE,
            "source_audit_kind": "committed implementation baseline; not moving HEAD",
            "source_audit_independently_reexecuted_by_this_pack": False,
            "current_rust": "1.96.1",
            "current_edition": "2021",
            "real_project_adr_owners": {
                "cp5": "0025",
                "llm_orchestration": "0033",
                "top_level_parked_tab_history": "0028",
                "public_ecosystem": "0029",
            },
            "blocking_documentation_issues": [],
            "files": entries,
        }
        (pack / "MANIFEST.json").write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
        )
        return pack

    @staticmethod
    def refresh_entry(pack: Path, relative: str) -> None:
        path = pack / "MANIFEST.json"
        manifest = json.loads(path.read_text(encoding="utf-8"))
        payload = (pack / relative).read_bytes()
        entry = next(item for item in manifest["files"] if item["path"] == relative)
        entry["bytes"] = len(payload)
        entry["sha256"] = hashlib.sha256(payload).hexdigest()
        path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    @staticmethod
    def valid_files() -> dict[str, str]:
        return {
            "README.md": (
                "# Pack\n\n"
                f"Audited implementation baseline: `{BASELINE}`.\n\n"
                "ADR 0025 is proposed. ADR 0028 owns top-level history. "
                "ADR 0029 and ADR 0033 are proposed.\n\n"
                "v0.4 release closure is separate from v0.5 activation hardening.\n"
            ),
            "repository_snapshot/CURRENT_REPOSITORY_STATE_2026-08-23.md": (
                "# State\n\n"
                f"Audited implementation baseline: `{BASELINE}`.\n"
            ),
            "research_proposals/ghostty/GHOSTTY_IMPLEMENTATION_STATUS_2026-08-23.md": (
                "# Ghostty status\n\n"
                f"Audited implementation baseline: `{BASELINE}`.\n"
            ),
            "historical/old.md": "# Historical\n",
        }

    def test_valid_pack_passes(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_pack(root, self.valid_files())
            counts = CHECKER.validate(root)
            self.assertEqual(counts["files"], 4)
            self.assertEqual(counts["historical"], 1)
            self.assertEqual(counts["duplicates"], 0)

    def test_hash_or_size_tampering_is_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            pack = self.create_pack(root, self.valid_files())
            with (pack / "README.md").open("a", encoding="utf-8") as target:
                target.write("tampered\n")
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "hash|size"):
                CHECKER.validate(root)

    def test_unlisted_file_is_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            pack = self.create_pack(root, self.valid_files())
            (pack / "unlisted.md").write_text("# Unlisted\n", encoding="utf-8")
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "inventory"):
                CHECKER.validate(root)

    def test_nonhistorical_duplicate_is_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            files = self.valid_files()
            files["duplicate.md"] = files["README.md"]
            self.create_pack(root, files)
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "duplicate"):
                CHECKER.validate(root)

    def test_stale_or_contradictory_claim_is_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            files = self.valid_files()
            files["research_proposals/conflict.md"] = (
                "# Conflict\n\nSandboxed extensions use Wasmtime and versioned WIT interfaces.\n"
            )
            self.create_pack(root, files)
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "forbidden"):
                CHECKER.validate(root)

    def test_duplicate_manifest_key_is_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            pack = self.create_pack(root, self.valid_files())
            manifest = (pack / "MANIFEST.json").read_text(encoding="utf-8")
            manifest = manifest.replace('"schema": 1,', '"schema": 1,\n  "schema": 1,', 1)
            (pack / "MANIFEST.json").write_text(manifest, encoding="utf-8")
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "duplicate key"):
                CHECKER.validate(root)


    def test_unexpected_manifest_entry_key_is_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            pack = self.create_pack(root, self.valid_files())
            path = pack / "MANIFEST.json"
            manifest = json.loads(path.read_text(encoding="utf-8"))
            manifest["files"][0]["unexpected"] = True
            path.write_text(
                json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(
                CHECKER.DocumentationPackError, "unexpected keys"
            ):
                CHECKER.validate(root)

    def test_non_utf8_markdown_is_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            pack = self.create_pack(root, self.valid_files())
            target = pack / "README.md"
            payload = b"\xff"
            target.write_bytes(payload)

            path = pack / "MANIFEST.json"
            manifest = json.loads(path.read_text(encoding="utf-8"))
            entry = next(item for item in manifest["files"] if item["path"] == "README.md")
            entry["bytes"] = len(payload)
            entry["sha256"] = hashlib.sha256(payload).hexdigest()
            path.write_text(
                json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "UTF-8"):
                CHECKER.validate(root)

    def test_multiple_active_h1_headings_are_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            pack = self.create_pack(root, self.valid_files())
            readme = pack / "README.md"
            readme.write_bytes((readme.read_text(encoding="utf-8") + "\n# Duplicate\n").encode("utf-8"))
            self.refresh_entry(pack, "README.md")
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "exactly one H1"):
                CHECKER.validate(root)

    def test_skipped_active_heading_level_is_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            pack = self.create_pack(root, self.valid_files())
            readme = pack / "README.md"
            readme.write_bytes((readme.read_text(encoding="utf-8") + "\n### Skipped\n").encode("utf-8"))
            self.refresh_entry(pack, "README.md")
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "skips"):
                CHECKER.validate(root)

    @staticmethod
    def create_private_boundary(root: Path) -> None:
        (root / ".gitignore").write_text(
            "/.automexia-private/\n", encoding="utf-8"
        )
        policy = root / CHECKER.POLICY_PATH
        policy.parent.mkdir(parents=True)
        policy.write_text(
            "# Boundary\n\n"
            "## Public documentation\n\nPublic behavior.\n\n"
            "## Local-only documentation\n\nUnreleased detail.\n\n"
            "## Never document in the repository\n\nNo secrets.\n\n"
            "## Publication review\n\nReview before sharing.\n",
            encoding="utf-8",
        )
        (policy.parent / "index.md").write_text(
            "# Documentation\n", encoding="utf-8"
        )

    def test_private_boundary_passes_and_skips_local_content(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_private_boundary(root)
            private = root / CHECKER.PRIVATE_PATH
            private.mkdir()
            (private / "internal.md").write_text(
                f"# Internal\n\n{CHECKER.PACK_PATH}\n", encoding="utf-8"
            )
            counts = CHECKER.validate(root)
            self.assertEqual(counts["files"], 2)
            self.assertEqual(counts["historical"], 0)
            self.assertEqual(counts["duplicates"], 0)

    def test_private_boundary_prunes_ignored_roots_before_scanning(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_private_boundary(root)
            excluded = {".automexia-private", ".automexia-tools", ".git", "target", "artifacts"}
            for name in excluded:
                cached = root / name / "nested" / "cached.md"
                cached.parent.mkdir(parents=True)
                cached.write_bytes(b"\xff")
            scandir = os.scandir
            visits = []

            def observe(path):
                relative = Path(path).relative_to(root)
                self.assertFalse(excluded.intersection(relative.parts), "private/tool/build trees must not be traversed")
                visits.append(relative.as_posix())
                return scandir(path)

            with mock.patch("os.scandir", side_effect=observe):
                self.assertEqual(CHECKER.validate(root)["files"], 2)
            self.assertCountEqual(visits, [".", "docs"])

    def test_private_boundary_fails_on_unreadable_public_directory(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_private_boundary(root)
            scandir = os.scandir

            def deny_public(path):
                if Path(path) == root / "docs":
                    raise PermissionError("fixture documentation denied")
                return scandir(path)

            with mock.patch("os.scandir", side_effect=deny_public):
                with self.assertRaisesRegex(CHECKER.DocumentationPackError, "cannot read a public documentation directory"):
                    CHECKER.validate(root)

    def test_private_boundary_still_checks_nested_public_target_names(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_private_boundary(root)
            public = root / "docs" / "target" / "public.md"
            public.parent.mkdir()
            public.write_text("# Future features\n", encoding="utf-8")
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "future planning"):
                CHECKER.validate(root)

    def test_private_boundary_rejects_legacy_public_pack(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_private_boundary(root)
            (root / CHECKER.PACK_PATH).mkdir()
            with self.assertRaisesRegex(
                CHECKER.DocumentationPackError, "must remain removed"
            ):
                CHECKER.validate(root)

    def test_private_boundary_rejects_negated_ignore_rule(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_private_boundary(root)
            with (root / ".gitignore").open("a", encoding="utf-8") as target:
                target.write("!/.automexia-private/README.md\n")
            with self.assertRaisesRegex(CHECKER.DocumentationPackError, "negated"):
                CHECKER.validate(root)

    def test_private_boundary_rejects_public_link_to_private_content(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_private_boundary(root)
            (root / "docs" / "index.md").write_text(
                "# Documentation\n\n[Internal](../.automexia-private/README.md)\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                CHECKER.DocumentationPackError, "links into the ignored"
            ):
                CHECKER.validate(root)

    def test_future_plans_are_rejected_even_under_renamed_public_pages(self) -> None:
        # The release merge restored proposal text in otherwise current-status
        # pages. Renaming a page or calling the feature free must not evade review.
        examples = (
            "## Direction after the first stable release\n\nA later free feature.\n",
            "## Future features\n\nAn open-source addition.\n",
            "Status: planned public direction.\n",
            "Status: proposed public summary; no implementation exists.\n",
            "| Optional component | Separate later direction only. |\n",
        )
        for relative in ("README.md", "docs/guide/renamed.md", "changes/review.md"):
            for example in examples:
                with self.subTest(path=relative, example=example.splitlines()[0]):
                    with TemporaryDirectory() as directory:
                        root = Path(directory)
                        self.create_private_boundary(root)
                        target = root / relative
                        target.parent.mkdir(parents=True, exist_ok=True)
                        target.write_text("# Current status\n\n" + example, encoding="utf-8")
                        with self.assertRaisesRegex(
                            CHECKER.DocumentationPackError, "future planning"
                        ) as failure:
                            CHECKER.validate(root)
                        self.assertNotIn(example.strip(), str(failure.exception))

    def test_current_behavior_and_external_release_gates_remain_public(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            self.create_private_boundary(root)
            (root / "README.md").write_text(
                "# Current status\n\n"
                "Tabs are implemented in source. Native release evidence is external.\n"
                "Future feature plans must not be published here.\n",
                encoding="utf-8",
            )
            self.assertEqual(CHECKER.validate(root)["files"], 3)

if __name__ == "__main__":
    unittest.main(verbosity=2)
