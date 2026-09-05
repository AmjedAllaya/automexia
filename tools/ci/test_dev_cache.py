#!/usr/bin/env python3
"""Mutation and lifecycle tests for the development-cache owner."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import time
import tomllib
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "automexia_dev_cache", ROOT / "tools/ci/dev_cache.py"
)
assert SPEC and SPEC.loader
CACHE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CACHE
SPEC.loader.exec_module(CACHE)


class DevelopmentCacheTests(unittest.TestCase):
    def candidate(
        self,
        path: Path,
        category: str,
        *,
        age_hours: int = 100,
        current: bool = False,
        clean: bool = True,
        leased: bool = False,
        current_toolset: bool = False,
    ) -> CACHE.Candidate:
        measurement = CACHE.Measurement(
            bytes=10,
            newest_mtime=time.time() - age_hours * 3600,
            files=1,
            directories=1,
        )
        return CACHE.Candidate(
            label=category,
            category=category,
            path=path,
            measurement=measurement,
            current=current,
            clean=clean,
            leased=leased,
            current_toolset=current_toolset,
        )

    def test_cargo_profiles_and_cache_layout_preserve_debug_opt_in(self) -> None:
        config = tomllib.loads((ROOT / ".cargo/config.toml").read_text(encoding="utf-8"))
        manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        self.assertEqual(config["build"]["build-dir"], "{workspace-root}/target/build")
        self.assertEqual(config["cache"]["auto-clean-frequency"], "1 day")
        self.assertEqual(manifest["profile"]["dev"]["debug"], "line-tables-only")
        self.assertFalse(manifest["profile"]["dev"]["package"]["*"]["debug"])
        self.assertEqual(manifest["profile"]["test"]["debug"], "line-tables-only")
        self.assertFalse(manifest["profile"]["test"]["incremental"])
        self.assertEqual(manifest["profile"]["debugging"]["debug"], "full")

    def test_toolset_identity_is_order_independent_and_platform_bound(self) -> None:
        first = {"semgrep": "1.0", "cargo-audit": "2.0"}
        second = dict(reversed(list(first.items())))
        with mock.patch.object(CACHE, "normalized_platform", return_value="test-os"), mock.patch.object(
            CACHE, "normalized_architecture", return_value="test-arch"
        ):
            self.assertEqual(CACHE.toolset_id(first), CACHE.toolset_id(second))
            changed_version = CACHE.toolset_id({**first, "semgrep": "1.1"})
        with mock.patch.object(CACHE, "normalized_platform", return_value="other-os"), mock.patch.object(
            CACHE, "normalized_architecture", return_value="test-arch"
        ):
            changed_platform = CACHE.toolset_id(first)
        self.assertNotEqual(changed_version, CACHE.toolset_id(first))
        self.assertNotEqual(changed_platform, CACHE.toolset_id(first))

    def test_cache_override_must_be_absolute_and_cannot_be_a_link(self) -> None:
        with mock.patch.dict(os.environ, {"AUTOMEXIA_DEV_CACHE_DIR": "relative"}, clear=False):
            with self.assertRaisesRegex(CACHE.CacheError, "absolute"):
                CACHE.cache_root()
        with tempfile.TemporaryDirectory() as temporary:
            link = Path(temporary) / "linked"
            link.mkdir()
            with mock.patch.dict(
                os.environ, {"AUTOMEXIA_DEV_CACHE_DIR": str(link)}, clear=False
            ), mock.patch.object(CACHE, "_is_reparse_or_link", return_value=True):
                with self.assertRaisesRegex(CACHE.CacheError, "link"):
                    CACHE.cache_root()

    def test_worktree_parser_preserves_spaces_and_rejects_relative_paths(self) -> None:
        separator = b"\0"
        first = os.fsencode(Path(tempfile.gettempdir()) / "alpha tree")
        second = os.fsencode(Path(tempfile.gettempdir()) / "unicode-tree")
        payload = (
            b"worktree " + first + separator + b"HEAD abc" + separator
            + b"worktree " + second + separator + b"HEAD def" + separator
        )
        self.assertEqual(
            CACHE.parse_worktree_porcelain(payload),
            [Path(os.path.abspath(os.fsdecode(first))), Path(os.path.abspath(os.fsdecode(second)))],
        )
        with self.assertRaisesRegex(CACHE.CacheError, "non-absolute"):
            CACHE.parse_worktree_porcelain(b"worktree relative\0")

    def test_tree_measurement_is_exact_and_rejects_links_and_file_floods(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "nested").mkdir()
            (root / "one").write_bytes(b"123")
            (root / "nested" / "two").write_bytes(b"45")
            measurement = CACHE.measure_tree(root)
            self.assertEqual(measurement.bytes, 5)
            self.assertEqual(measurement.files, 2)
            self.assertEqual(measurement.directories, 2)
            with mock.patch.object(CACHE, "MAX_FILES", 1):
                with self.assertRaisesRegex(CACHE.CacheError, "file count"):
                    CACHE.measure_tree(root)
            link = root / "link"
            try:
                link.symlink_to(root / "one")
            except OSError:
                return
            with self.assertRaisesRegex(CACHE.CacheError, "links"):
                CACHE.measure_tree(root)

    def test_automatic_inventory_does_not_walk_normal_worktree_targets(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "checkout"
            (root / "target" / "debug").mkdir(parents=True)
            (root / "target" / "debug" / "app").write_bytes(b"binary")
            # Cargo owns ordinary root metadata beside the directories; the
            # cache manager must ignore it without relaxing link checks.
            (root / "target" / ".rustc_info.json").write_text("{}", encoding="utf-8")
            cache = Path(temporary) / "cache"
            with mock.patch.object(CACHE, "worktree_paths", return_value=[root]), mock.patch.object(
                CACHE, "cache_root", return_value=cache
            ), mock.patch.object(CACHE, "measure_tree", wraps=CACHE.measure_tree) as measure:
                candidates = CACHE.inventory(root=root, scope="automatic")
            self.assertEqual(candidates, [])
            measure.assert_not_called()

    def test_current_worktree_verification_artifacts_are_automatically_reclaimable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "checkout"
            verification = root / "target" / "automexia-verification-v1-1-1"
            verification.mkdir(parents=True)
            artifact = verification / "artifact"
            artifact.write_bytes(b"generated")
            old = time.time() - 100 * 3600
            os.utime(artifact, (old, old))
            os.utime(verification, (old, old))
            cache = Path(temporary) / "cache"
            with mock.patch.object(CACHE, "worktree_paths", return_value=[root]), mock.patch.object(
                CACHE, "cache_root", return_value=cache
            ):
                candidates = CACHE.inventory(root=root, scope="automatic")
            self.assertEqual(len(candidates), 1)
            self.assertFalse(candidates[0].current)
            self.assertEqual(
                CACHE.reclaimable(candidates, scope="automatic", grace_hours=72),
                candidates,
            )

    def test_current_worktree_legacy_tool_cache_requires_explicit_tool_scope(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "checkout"
            legacy = root / CACHE.LEGACY_TOOL_DIRECTORY_NAME
            legacy.mkdir(parents=True)
            artifact = legacy / "generated-tool"
            artifact.write_bytes(b"generated")
            old = time.time() - 100 * 3600
            os.utime(artifact, (old, old))
            os.utime(legacy, (old, old))
            cache = Path(temporary) / "cache"
            with mock.patch.object(CACHE, "worktree_paths", return_value=[root]), mock.patch.object(
                CACHE, "cache_root", return_value=cache
            ):
                automatic = CACHE.inventory(root=root, scope="automatic")
                candidates = CACHE.inventory(root=root, scope="tools")
            self.assertEqual(automatic, [])
            self.assertEqual([candidate.category for candidate in candidates], ["legacy-tool"])
            self.assertFalse(candidates[0].current)
            self.assertEqual(
                CACHE.reclaimable(candidates, scope="tools", grace_hours=72),
                candidates,
            )

    def test_gc_protects_current_dirty_leased_recent_and_required_entries(self) -> None:
        root = Path(tempfile.gettempdir())
        candidates = [
            self.candidate(root / "current", "other-target", current=True),
            self.candidate(root / "dirty", "other-target", clean=False),
            self.candidate(root / "leased", "other-target", leased=True),
            self.candidate(root / "recent", "other-target", age_hours=1),
            self.candidate(
                root / "required",
                "old-toolset",
                current_toolset=True,
            ),
            self.candidate(root / "eligible", "other-target"),
        ]
        selected = CACHE.reclaimable(
            candidates, scope="worktrees", grace_hours=72
        )
        self.assertEqual([item.path for item in selected], [root / "eligible"])

    def test_scope_selection_is_explicit_and_all_deduplicates_nested_paths(self) -> None:
        root = Path(tempfile.gettempdir()) / "scope"
        target = self.candidate(root / "target", "other-target")
        nested = self.candidate(
            root / "target" / "automexia-verification-v1-1-1",
            "stale-verification",
        )
        legacy = self.candidate(root / "legacy", "legacy-tool")
        old_toolset = self.candidate(root / "old-toolset", "old-toolset")
        temporary = self.candidate(root / "temporary", "temporary")
        self.assertEqual(
            {
                item.path
                for item in CACHE.reclaimable(
                    [target, nested, legacy, old_toolset, temporary],
                    scope="automatic",
                    grace_hours=72,
                )
            },
            {nested.path, old_toolset.path, temporary.path},
        )
        self.assertEqual(
            CACHE.reclaimable(
                [target, nested, legacy, old_toolset, temporary],
                scope="tools",
                grace_hours=72,
            ),
            [legacy, old_toolset],
        )
        self.assertEqual(
            {
                item.path
                for item in CACHE.reclaimable(
                    [target, nested, legacy, old_toolset, temporary],
                    scope="all",
                    grace_hours=72,
                )
            },
            {target.path, legacy.path, old_toolset.path, temporary.path},
        )

    def test_invalid_scope_and_grace_fail_closed(self) -> None:
        with self.assertRaisesRegex(CACHE.CacheError, "scope"):
            CACHE.reclaimable([], scope="unknown", grace_hours=72)
        for value in (-1, 24 * 365 + 1):
            with self.subTest(value=value):
                with self.assertRaisesRegex(CACHE.CacheError, "0..8760"):
                    CACHE.reclaimable([], scope="all", grace_hours=value)

    def test_shared_lease_is_visible_and_released(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "checkout"
            root.mkdir()
            cache = Path(temporary) / "cache"
            with mock.patch.object(CACHE, "cache_root", return_value=cache):
                self.assertFalse(CACHE.shared_cache_is_leased(root=root))
                with CACHE.cache_lease("assurance", root=root):
                    self.assertTrue(CACHE.shared_cache_is_leased(root=root))
                self.assertFalse(CACHE.shared_cache_is_leased(root=root))

    def test_activity_marker_protects_live_process_but_not_a_dead_owner(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary)
            marker = target / CACHE.ACTIVE_MARKER_NAME
            marker.write_text(str(os.getpid()), encoding="ascii")
            self.assertTrue(CACHE._active_marker(target))
            marker.write_text("2147483647", encoding="ascii")
            with mock.patch.object(CACHE, "process_is_active", return_value=False):
                self.assertFalse(CACHE._active_marker(target))
            marker.write_text("malformed", encoding="ascii")
            self.assertTrue(CACHE._active_marker(target))

    def test_native_process_probe_distinguishes_running_and_exited_children(self) -> None:
        child = subprocess.Popen(
            [sys.executable, "-c", "input()"],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        try:
            self.assertTrue(CACHE.process_is_active(child.pid))
        finally:
            child.terminate()
            assert child.stdin is not None
            child.stdin.close()
            child.wait(timeout=10)
        self.assertFalse(CACHE.process_is_active(child.pid))

    def test_removal_refuses_unknown_paths_and_removes_only_exact_targets(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "checkout"
            target = root / "target"
            target.mkdir(parents=True)
            artifact = target / "artifact"
            artifact.write_bytes(b"x")
            candidate = self.candidate(target, "other-target")
            outside = self.candidate(Path(temporary) / "outside", "other-target")
            outside.path.mkdir()
            with mock.patch.object(CACHE, "worktree_paths", return_value=[root]):
                with self.assertRaisesRegex(CACHE.CacheError, "outside"):
                    CACHE.remove_candidate(outside, root=root)
                CACHE.remove_candidate(candidate, root=root)
            self.assertFalse(target.exists())
            self.assertTrue(outside.path.exists())

    def test_windows_readonly_retry_is_bounded_to_the_requested_entry(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "readonly"
            path.write_bytes(b"x")
            path.chmod(stat.S_IREAD)
            removed: list[str] = []

            def remove(value: str) -> None:
                removed.append(value)

            CACHE._retry_readonly_removal(
                remove,
                str(path),
                (PermissionError, PermissionError("blocked"), None),
            )
            self.assertEqual(removed, [str(path)])

    def test_status_warns_without_treating_overlapping_rows_as_extra_bytes(self) -> None:
        root = Path(tempfile.gettempdir()) / "status"
        parent = self.candidate(root, "other-target")
        nested = self.candidate(root / "nested", "stale-verification")
        parent = CACHE.Candidate(
            **{**parent.__dict__, "measurement": CACHE.Measurement(CACHE.GIB, 1, 1, 1)}
        )
        nested = CACHE.Candidate(
            **{**nested.__dict__, "measurement": CACHE.Measurement(CACHE.GIB, 1, 1, 1)}
        )
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            CACHE.print_status([parent, nested], warn_gib=2)
        self.assertIn("below its warning budget", output.getvalue())


if __name__ == "__main__":
    unittest.main(verbosity=2)
