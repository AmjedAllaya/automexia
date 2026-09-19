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
    @contextlib.contextmanager
    def collection_fixture(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "checkout"
            root.mkdir()
            cache = Path(temporary) / "cache"
            obsolete = cache / "toolsets" / "obsolete"
            obsolete.mkdir(parents=True)
            (obsolete / "artifact").write_bytes(b"fixture")
            with (
                mock.patch.object(CACHE, "cache_root", return_value=cache),
                mock.patch.object(CACHE, "worktree_paths", return_value=[]),
                mock.patch.object(CACHE, "_current_toolset_id", return_value="current"),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                yield root, cache, obsolete

    def collect(self, root: Path, *, apply: bool = True) -> int:
        return CACHE.run_gc(root=root, scope="automatic", grace_hours=0, apply=apply)

    def test_collection_excludes_new_lease_until_deletion_finishes(self) -> None:
        with self.collection_fixture() as (root, _cache, obsolete):
            remove = CACHE.remove_candidate

            def checked_remove(candidate, *, root):
                with self.assertRaises(CACHE.CacheError):
                    with CACHE.cache_lease("late-writer", root=root):
                        self.fail("a writer entered while collection could delete its cache")
                remove(candidate, root=root)

            with mock.patch.object(CACHE, "remove_candidate", side_effect=checked_remove) as deletion:
                self.assertEqual(self.collect(root), len(b"fixture"))
            self.assertEqual(deletion.call_count, 1)
            self.assertFalse(obsolete.exists())
            with CACHE.cache_lease("late-writer", root=root):
                pass

    def test_collection_retains_idle_named_lease_until_deletion_finishes(self) -> None:
        with self.collection_fixture() as (root, cache, obsolete):
            with CACHE.cache_lease("existing-writer", root=root):
                pass
            remove = CACHE.remove_candidate

            def checked_remove(candidate, *, root):
                path = cache / CACHE.LEASE_DIRECTORY_NAME / "existing-writer.lock"
                with path.open("r+b") as contender:
                    with self.assertRaises(OSError):
                        CACHE._lock(contender, nonblocking=True)
                remove(candidate, root=root)

            with mock.patch.object(CACHE, "remove_candidate", side_effect=checked_remove):
                self.collect(root)
            self.assertFalse(obsolete.exists())

    def test_collection_preserves_active_leases_and_distinct_writers(self) -> None:
        with self.collection_fixture() as (root, _cache, obsolete):
            with CACHE.cache_lease("first", root=root), CACHE.cache_lease("second", root=root):
                self.assertEqual(self.collect(root), 0)
                self.assertEqual((obsolete / "artifact").read_bytes(), b"fixture")
            self.assertEqual(self.collect(root), len(b"fixture"))

    def test_collection_rejects_another_collector_and_releases_after_failure(self) -> None:
        with self.collection_fixture() as (root, _cache, obsolete):
            attempts = []

            def failed_remove(candidate, *, root):
                attempts.append(candidate)
                self.assertEqual(len(attempts), 1, "a second collector reached deletion")
                with self.assertRaises(CACHE.CacheError):
                    self.collect(root)
                raise CACHE.CacheError("fixture removal failure")

            with mock.patch.object(CACHE, "remove_candidate", side_effect=failed_remove):
                with self.assertRaisesRegex(CACHE.CacheError, "fixture removal failure"):
                    self.collect(root)
            self.assertTrue(obsolete.exists())
            with CACHE.cache_lease("after-error", root=root):
                pass
            self.assertEqual(self.collect(root), len(b"fixture"))

    def test_collection_native_subprocess_cannot_enter_during_deletion(self) -> None:
        with self.collection_fixture() as (root, cache, obsolete):
            remove = CACHE.remove_candidate
            probe = "\n".join([
                "import importlib.util, pathlib, sys",
                "spec = importlib.util.spec_from_file_location('cache_probe', sys.argv[1])",
                "owner = importlib.util.module_from_spec(spec)",
                "sys.modules[spec.name] = owner",
                "spec.loader.exec_module(owner)",
                "owner.cache_root = lambda **kwargs: pathlib.Path(sys.argv[2])",
                "try:",
                "    with owner.cache_lease('child-probe', root=pathlib.Path(sys.argv[3])):",
                "        pass",
                "except owner.CacheError:",
                "    sys.exit(42)",
            ])

            def checked_remove(candidate, *, root):
                result = subprocess.run(
                    [sys.executable, "-B", "-c", probe, str(ROOT / "tools/ci/dev_cache.py"), str(cache), str(root)],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=10,
                    check=False,
                )
                self.assertEqual(result.returncode, 42)
                remove(candidate, root=root)

            with mock.patch.object(CACHE, "remove_candidate", side_effect=checked_remove):
                self.collect(root)
            self.assertFalse(obsolete.exists())

    def test_collection_dry_run_creates_no_lease_files(self) -> None:
        with self.collection_fixture() as (root, cache, obsolete):
            before = sorted(path.relative_to(cache) for path in cache.rglob("*"))
            self.assertEqual(self.collect(root, apply=False), len(b"fixture"))
            self.assertEqual(sorted(path.relative_to(cache) for path in cache.rglob("*")), before)
            self.assertEqual((obsolete / "artifact").read_bytes(), b"fixture")

    def test_tree_scan_stops_before_materializing_file_and_directory_floods(self) -> None:
        class CountedScan:
            def __init__(self, entries):
                self.entries = iter(entries)
                self.visits = 0
                self.closed = False

            def __enter__(self):
                return self

            def __exit__(self, *args):
                self.closed = True

            def __iter__(self):
                return self

            def __next__(self):
                entry = next(self.entries)
                self.visits += 1
                return entry

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for directories in (False, True):
                with self.subTest(directories=directories):
                    fixture = root / ("directories" if directories else "files")
                    fixture.mkdir()
                    if directories:
                        (fixture / "entry").mkdir()
                    else:
                        (fixture / "entry").write_bytes(b"x")
                    with os.scandir(fixture) as entries:
                        entry = next(entries)
                    scan = CountedScan([entry] * 100)
                    with (
                        mock.patch.object(CACHE.os, "scandir", return_value=scan),
                        mock.patch.object(CACHE, "MAX_FILES", 2),
                        mock.patch.object(CACHE, "MAX_DIRECTORIES", 2),
                    ):
                        with self.assertRaises(CACHE.CacheError):
                            CACHE.measure_tree(fixture)
                    self.assertEqual(scan.visits, 2 if directories else 3)
                    self.assertTrue(scan.closed)

    def test_collection_scan_stops_at_entry_limit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for index in range(10):
                (root / str(index)).mkdir()
            original_scan = os.scandir
            original_iterdir = Path.iterdir
            visits = []

            def counted(entries):
                for entry in entries:
                    visits.append(1)
                    yield entry

            @contextlib.contextmanager
            def scan(path):
                with original_scan(path) as entries:
                    yield counted(entries)

            with (
                mock.patch.object(CACHE.os, "scandir", side_effect=scan),
                mock.patch.object(Path, "iterdir", side_effect=lambda: counted(original_iterdir(root))),
                mock.patch.object(CACHE, "MAX_DIRECTORIES", 2),
            ):
                with self.assertRaisesRegex(CACHE.CacheError, "ceiling"):
                    CACHE._direct_directories(root)
            self.assertEqual(len(visits), 3)

    def test_collection_inventory_error_releases_admission_and_named_locks(self) -> None:
        with self.collection_fixture() as (root, _cache, obsolete):
            with CACHE.cache_lease("existing", root=root):
                pass
            with mock.patch.object(CACHE, "inventory", side_effect=CACHE.CacheError("fixture inventory failure")):
                with self.assertRaisesRegex(CACHE.CacheError, "fixture inventory failure"):
                    self.collect(root)
            self.assertEqual((obsolete / "artifact").read_bytes(), b"fixture")
            with CACHE.cache_lease("existing", root=root), CACHE.cache_lease("new", root=root):
                pass
            self.assertEqual(self.collect(root), len(b"fixture"))

    def test_lease_scan_bounds_handles_and_releases_on_overflow(self) -> None:
        with self.collection_fixture() as (root, cache, obsolete):
            for name in ("first", "second", "third"):
                with CACHE.cache_lease(name, root=root):
                    pass
            with mock.patch.object(CACHE, "MAX_LEASES", 2):
                with self.assertRaisesRegex(CACHE.CacheError, "ceiling"):
                    self.collect(root)
            self.assertEqual((obsolete / "artifact").read_bytes(), b"fixture")
            for name in ("first", "second", "third"):
                path = cache / CACHE.LEASE_DIRECTORY_NAME / f"{name}.lock"
                with path.open("r+b") as handle:
                    CACHE._lock(handle, nonblocking=True)
                    CACHE._unlock(handle)
            with CACHE.cache_lease("after-overflow", root=root):
                pass
            self.assertEqual(self.collect(root), len(b"fixture"))

    def test_lease_scan_bounds_ignored_entries_and_closes_iterator(self) -> None:
        with self.collection_fixture() as (root, cache, _obsolete):
            directory = cache / CACHE.LEASE_DIRECTORY_NAME
            directory.mkdir()
            for index in range(10):
                (directory / f"ignored-{index}").write_bytes(b"")
            original_scan = os.scandir
            visits = []
            closed = []

            @contextlib.contextmanager
            def scan(path):
                with original_scan(path) as entries:
                    def counted():
                        for entry in entries:
                            visits.append(1)
                            yield entry
                    try:
                        yield counted()
                    finally:
                        closed.append(True)

            with (
                mock.patch.object(CACHE.os, "scandir", side_effect=scan),
                mock.patch.object(CACHE, "MAX_DIRECTORIES", 2),
            ):
                with self.assertRaisesRegex(CACHE.CacheError, "ceiling"):
                    CACHE.shared_cache_is_leased(root=root)
            self.assertEqual(len(visits), 3)
            self.assertEqual(closed, [True])

    def test_streaming_tree_accepts_exact_limits_and_rejects_next_entry(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "child").mkdir()
            (root / "child" / "first").write_bytes(b"one")
            with (
                mock.patch.object(CACHE, "MAX_FILES", 1),
                mock.patch.object(CACHE, "MAX_DIRECTORIES", 2),
            ):
                measured = CACHE.measure_tree(root)
                self.assertEqual((measured.bytes, measured.files, measured.directories), (3, 1, 2))
                (root / "second").write_bytes(b"two")
                with self.assertRaisesRegex(CACHE.CacheError, "file count"):
                    CACHE.measure_tree(root)
            (root / "child" / "nested").mkdir()
            with mock.patch.object(CACHE, "MAX_DIRECTORIES", 2):
                with self.assertRaisesRegex(CACHE.CacheError, "directory count"):
                    CACHE.measure_tree(root)

    def test_lease_names_are_bounded_before_creating_files(self) -> None:
        with self.collection_fixture() as (root, cache, _obsolete):
            for name in ("", "UPPER", "../outside", "a" * 65):
                with self.subTest(name=name), self.assertRaisesRegex(CACHE.CacheError, "identifier"):
                    with CACHE.cache_lease(name, root=root):
                        self.fail("invalid lease was admitted")
            self.assertFalse((cache / CACHE.LEASE_DIRECTORY_NAME).exists())
            with CACHE.cache_lease("a" * 64, root=root):
                pass

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
