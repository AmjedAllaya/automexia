"""Offline, harmless PE/archive fixtures for the actual runtime producer."""

import copy
import hashlib
import io
import json
import os
from pathlib import Path
import socket
import stat
import struct
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock
import zipfile

import windows_conpty_runtime as runtime
import qa_process


def pe(machine, suffix=b""):
    data = bytearray(70)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 60, 64)
    data[64:68] = b"PE\0\0"
    struct.pack_into("<H", data, 68, machine)
    return bytes(data) + suffix


class RuntimePreparationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="conpty-unit-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.destination = self.root / "application"
        self.cache = self.root / "cache"
        self.recipe = copy.deepcopy(runtime.load_recipe())
        self.payloads = {}
        for item in self.recipe["files"]:
            data = pe(item["machine"], item["member"].encode("ascii"))
            item["size"] = len(data)
            item["sha256"] = hashlib.sha256(data).hexdigest()
            self.payloads[item["member"]] = data
        self.archive = self.root / "fixture.nupkg"
        self.write_archive()
        self.addCleanup(mock.patch.stopall)
        mock.patch.object(runtime, "load_recipe", return_value=self.recipe).start()
        mock.patch.dict(os.environ, {"AUTOMEXIA_DEV_CACHE_DIR": str(self.cache)}).start()
        self.network = mock.patch.object(runtime.urllib.request, "urlopen", side_effect=AssertionError("unexpected network")).start()

    def write_archive(self, extra=None):
        with zipfile.ZipFile(self.archive, "w") as archive:
            for name, data in self.payloads.items():
                archive.writestr(name, data)
            if extra:
                extra(archive)
        self.recipe["package"]["sha256"] = hashlib.sha256(self.archive.read_bytes()).hexdigest()

    def prepare(self, architecture="x64"):
        return runtime.prepare_runtime(self.destination, architecture, offline=True, archive=self.archive)

    def test_both_app_architectures_install_both_kernel_hosts_without_root_host(self):
        for architecture in ("x64", "arm64"):
            destination = self.root / architecture
            paths = runtime.prepare_runtime(destination, architecture, offline=True, archive=self.archive)
            self.assertEqual({p.relative_to(destination).as_posix() for p in paths},
                             {"conpty.dll", "x64/OpenConsole.exe", "arm64/OpenConsole.exe"})
            self.assertEqual(runtime.pe_architecture(destination / "conpty.dll"), architecture)
            for kernel in ("x64", "arm64"):
                self.assertEqual(runtime.pe_architecture(destination / kernel / "OpenConsole.exe"), kernel)
            self.assertFalse((destination / "OpenConsole.exe").exists())
        self.network.assert_not_called()

    def test_verified_cache_supports_offline_prepare_after_archive_is_removed(self):
        self.prepare()
        self.archive.unlink()
        paths = runtime.prepare_runtime(self.root / "second", "x64", offline=True)
        self.assertEqual(len(paths), 3)
        self.network.assert_not_called()

    def test_offline_miss_is_actionable_and_publishes_no_dll(self):
        with self.assertRaisesRegex(runtime.RuntimeError, "not cached"):
            runtime.prepare_runtime(self.destination, "x64", offline=True)
        self.assertFalse((self.destination / "conpty.dll").exists())
        self.network.assert_not_called()

    def test_archive_hash_failure_prevents_all_publication(self):
        self.archive.write_bytes(self.archive.read_bytes() + b"changed")
        with self.assertRaisesRegex(runtime.RuntimeError, "checksum"):
            self.prepare()
        self.assertFalse((self.destination / "conpty.dll").exists())

    def test_each_member_hash_is_required_even_for_other_app_architecture(self):
        for item in self.recipe["files"]:
            with self.subTest(member=item["member"]):
                original = item["sha256"]
                item["sha256"] = "0" * 64
                with self.assertRaisesRegex(runtime.RuntimeError, "safely verified"):
                    self.prepare()
                self.assertFalse((self.destination / "conpty.dll").exists())
                item["sha256"] = original

    def test_missing_member_is_rejected(self):
        self.payloads.pop(self.recipe["files"][-1]["member"])
        self.write_archive()
        with self.assertRaises(runtime.RuntimeError):
            self.prepare()

    def test_member_count_and_expanded_size_bounds_are_enforced(self):
        self.write_archive(lambda z: [z.writestr(f"extra-{index}", b"x")
                                    for index in range(runtime.MAX_ARCHIVE_MEMBERS)])
        with self.assertRaises(runtime.RuntimeError):
            self.prepare()
        self.write_archive()
        with mock.patch.object(runtime, "MAX_EXPANDED_BYTES", 1), self.assertRaises(runtime.RuntimeError):
            self.prepare()

    def test_download_cache_is_verified_before_extraction(self):
        self.prepare()
        runtime_cache = runtime.dev_cache.runtime_root() / "conpty" / self.recipe["package"]["sha256"]
        # Remove only this fixture-owned cache so the producer must use its retained archive.
        import shutil
        shutil.rmtree(runtime_cache)
        paths = runtime.prepare_runtime(self.root / "second", "x64", offline=True)
        self.assertEqual(len(paths), 3)
        self.network.assert_not_called()

    def test_unsafe_duplicate_and_oversized_archive_entries_are_rejected(self):
        def symlink(archive):
            entry = zipfile.ZipInfo("linked")
            entry.external_attr = (stat.S_IFLNK | 0o777) << 16
            archive.writestr(entry, "outside")
        cases = {
            "traversal": lambda z: z.writestr("../outside", b"bad"),
            "absolute": lambda z: z.writestr("/outside", b"bad"),
            "duplicate": lambda z: z.writestr(next(iter(self.payloads)).upper(), b"bad"),
            "symlink": symlink,
            "oversized": lambda z: z.writestr("oversized", b"x" * (runtime.MAX_MEMBER_BYTES + 1)),
        }
        for label, extra in cases.items():
            with self.subTest(case=label):
                self.write_archive(extra)
                with self.assertRaises(runtime.RuntimeError):
                    self.prepare()
                self.assertFalse((self.destination / "conpty.dll").exists())

    def test_machine_mismatch_fails_even_when_hash_matches(self):
        item = self.recipe["files"][0]
        data = pe(runtime.ARCHITECTURES["arm64"], b"wrong architecture")
        self.payloads[item["member"]] = data
        item["size"] = len(data)
        item["sha256"] = hashlib.sha256(data).hexdigest()
        self.write_archive()
        with self.assertRaises(runtime.RuntimeError):
            self.prepare()

    def test_conflicting_and_unknown_files_are_preserved(self):
        self.destination.mkdir()
        existing = self.destination / "conpty.dll"
        existing.write_bytes(b"user-owned")
        unrelated = self.destination / "settings.toml"
        unrelated.write_text("saved=true")
        with self.assertRaisesRegex(runtime.RuntimeError, "Conflicting"):
            self.prepare()
        self.assertEqual(existing.read_bytes(), b"user-owned")
        self.assertEqual(unrelated.read_text(), "saved=true")
        self.assertFalse((self.destination / "x64").exists())

    def test_root_host_override_is_rejected_without_removing_it(self):
        self.destination.mkdir()
        host = self.destination / "OpenConsole.exe"
        host.write_bytes(b"existing")
        with self.assertRaisesRegex(runtime.RuntimeError, "Root OpenConsole"):
            self.prepare()
        self.assertEqual(host.read_bytes(), b"existing")

    def test_identical_in_use_files_are_not_replaced_or_opened_for_writing(self):
        paths = self.prepare()
        identities = [(p.stat().st_ino, p.stat().st_mtime_ns) for p in paths]
        with (self.destination / "conpty.dll").open("rb"), mock.patch.object(runtime.os, "link", side_effect=AssertionError("republication")):
            self.assertEqual(self.prepare(), paths)
        self.assertEqual(identities, [(p.stat().st_ino, p.stat().st_mtime_ns) for p in paths])

    def test_dll_is_published_after_both_verified_hosts(self):
        publish = runtime.os.link
        order = []
        def check(source, target):
            order.append(target.relative_to(self.destination).as_posix())
            if target.name == "conpty.dll":
                for arch in ("x64", "arm64"):
                    self.assertTrue((self.destination / arch / "OpenConsole.exe").is_file())
            publish(source, target)
        with mock.patch.object(runtime.os, "link", side_effect=check):
            self.prepare()
        self.assertEqual(order[-1], "conpty.dll")

    def test_failed_host_publication_never_publishes_the_dll(self):
        publish = runtime.os.link
        def fail(source, target):
            if target.parent.name == "arm64":
                raise PermissionError("fixture")
            publish(source, target)
        with mock.patch.object(runtime.os, "link", side_effect=fail), self.assertRaisesRegex(runtime.RuntimeError, "publish"):
            self.prepare()
        self.assertFalse((self.destination / "conpty.dll").exists())
        self.prepare()  # A retry repairs the partial pair without replacing the first host.

    def test_corrupt_cache_fails_without_silently_redownloading(self):
        self.prepare()
        cache = runtime.dev_cache.runtime_root() / "conpty" / self.recipe["package"]["sha256"]
        (cache / self.recipe["files"][-1]["member"]).write_bytes(b"corrupt")
        with self.assertRaises(runtime.RuntimeError):
            runtime.prepare_runtime(self.root / "second", "x64", offline=True)
        self.network.assert_not_called()

    def test_existing_cache_lease_owner_covers_all_preparation(self):
        with mock.patch.object(runtime.dev_cache, "cache_lease", wraps=runtime.dev_cache.cache_lease) as lease:
            self.prepare()
        lease.assert_called_once_with("windows-conpty-runtime", root=runtime.ROOT)

    def test_concurrent_producers_share_one_complete_cache_and_destination(self):
        fixture_recipe = self.root / "recipe.json"
        fixture_recipe.write_text(json.dumps(self.recipe), encoding="utf-8")
        code = ("from pathlib import Path; import sys; import windows_conpty_runtime as r; "
                "r.RECIPE=Path(sys.argv[1]); "
                "r.prepare_runtime(Path(sys.argv[2]), 'x64', offline=True, archive=Path(sys.argv[3]))")
        command = [sys.executable, "-c", code, str(fixture_recipe), str(self.destination), str(self.archive)]
        children = []
        try:
            for _ in range(2):
                children.append(subprocess.Popen(command, cwd=Path(runtime.__file__).parent,
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0))
            for child in children:
                _, error = child.communicate(timeout=20)
                self.assertEqual(child.returncode, 0, error.decode("utf-8", errors="replace"))
        finally:
            for child in children:
                if child.poll() is None:
                    child.kill()
                child.communicate(timeout=5)
        self.assertEqual(len(runtime.verify_runtime(self.destination, "x64")), 3)

    def test_windows_reparse_attribute_is_rejected_even_without_symlink_privilege(self):
        original = Path.lstat
        destination = self.destination
        def lstat(path, *args, **kwargs):
            if path == destination:
                return mock.Mock(st_mode=stat.S_IFDIR, st_file_attributes=0x400)
            return original(path, *args, **kwargs)
        with mock.patch.object(Path, "lstat", lstat), self.assertRaisesRegex(runtime.RuntimeError, "links or reparse"):
            self.prepare()
        self.assertFalse(self.destination.exists())

    def test_dangling_root_host_reparse_point_cannot_override_kernel_selection(self):
        original = Path.lstat
        root_host = self.destination / "OpenConsole.exe"
        def lstat(path, *args, **kwargs):
            if path == root_host:
                return mock.Mock(st_mode=stat.S_IFREG, st_file_attributes=0x400)
            return original(path, *args, **kwargs)
        with mock.patch.object(Path, "lstat", lstat), self.assertRaisesRegex(runtime.RuntimeError, "links or reparse"):
            self.prepare()
        self.assertFalse((self.destination / "conpty.dll").exists())

    def test_bounded_download_verifies_bytes_before_using_them(self):
        contents = self.archive.read_bytes()
        class Response(io.BytesIO):
            headers = {"Content-Length": str(len(contents))}
            def geturl(inner):
                return self.recipe["package"]["url"]
        self.network.side_effect = lambda *args, **kwargs: Response(contents)
        with mock.patch.object(qa_process, "run", side_effect=self.download_worker):
            self.assertEqual(len(runtime.prepare_runtime(self.destination, "x64")), 3)
        self.network.assert_called_once_with(self.recipe["package"]["url"], timeout=30)

    def download_worker(self, command, **kwargs):
        self.assertEqual(command[:3], [sys.executable, str(Path(runtime.__file__).resolve()), "--download-worker"])
        self.assertEqual(kwargs["timeout_seconds"], runtime.DOWNLOAD_SECONDS)
        self.assertEqual(kwargs["cwd"], runtime.ROOT)
        runtime._download_worker(Path(command[-1]))
        return qa_process.Result(0, False, None)

    def test_download_rejects_redirects_length_and_actual_stream_overflow(self):
        for issue in ("redirect", "length", "stream"):
            with self.subTest(issue=issue):
                class Response(io.BytesIO):
                    headers = {"Content-Length": str(runtime.MAX_ARCHIVE_BYTES + 1)} if issue == "length" else {}
                    def geturl(inner):
                        return "https://unexpected.invalid/file" if issue == "redirect" else self.recipe["package"]["url"]
                data = b"x" * (runtime.MAX_ARCHIVE_BYTES + 1) if issue == "stream" else b"x"
                self.network.side_effect = lambda *args, **kwargs: Response(data)
                with mock.patch.object(qa_process, "run", side_effect=self.download_worker), self.assertRaises(runtime.RuntimeError):
                    runtime.prepare_runtime(self.destination, "x64")
                self.assertFalse((self.destination / "conpty.dll").exists())

    def test_download_deadline_is_bounded(self):
        class Response(io.BytesIO):
            headers = {}
            def geturl(inner):
                return self.recipe["package"]["url"]
        self.network.side_effect = lambda *args, **kwargs: Response(b"fixture")
        with mock.patch.object(runtime.time, "monotonic", side_effect=(0, runtime.DOWNLOAD_SECONDS + 1)):
            with self.assertRaisesRegex(runtime.RuntimeError, "time ceiling"):
                runtime._download_stream(self.recipe["package"]["url"], self.root / "download")

    def test_worker_rejects_unowned_destination_and_url_override(self):
        for destination in (self.root / "runtime.nupkg", self.root / "conpty-abcdefgh/runtime.nupkg",
                            runtime.dev_cache.staging_root(root=runtime.ROOT) / "conpty-abcdefgh/other"):
            with self.subTest(destination=destination.name):
                with self.assertRaisesRegex(runtime.RuntimeError, "owned staging"):
                    runtime._download_worker(destination)
        with mock.patch.object(qa_process, "run") as launch:
            with self.assertRaisesRegex(runtime.RuntimeError, "pinned package origin"):
                runtime._download("https://untrusted.invalid/package", self.root / "runtime.nupkg")
            self.assertEqual(runtime.main(["--download-worker", "--url", "https://untrusted.invalid/package"]), 2)
            launch.assert_not_called()
        self.network.assert_not_called()

    def test_retired_download_failure_cleans_stage_and_releases_lease(self):
        def retired_worker(command, **kwargs):
            Path(command[-1]).write_bytes(b"unfinished fixture")
            return qa_process.Result(None, True, "private child error")
        with mock.patch.object(qa_process, "run", side_effect=retired_worker):
            with self.assertRaisesRegex(runtime.RuntimeError, "within its deadline") as failure:
                runtime.prepare_runtime(self.destination, "x64")
        self.assertNotIn("private", str(failure.exception))
        self.assertFalse(list(runtime.dev_cache.staging_root(root=runtime.ROOT).glob("conpty-*")))
        self.assertIsNone(runtime._preparation_quarantine)
        self.assertEqual(len(self.prepare()), 3)

    def test_failed_download_cleanup_retains_stage_and_cache_lease(self):
        stages = []
        def unresolved_worker(command, **kwargs):
            stage = Path(command[-1]).parent
            stages.append(stage)
            (stage / "runtime.nupkg").write_bytes(b"unfinished fixture")
            kwargs["consume"](b"private child diagnostics must be discarded")
            return qa_process.Result(None, True, "QA native cleanup is incomplete; further launches blocked")
        try:
            with mock.patch.object(qa_process, "run", side_effect=unresolved_worker) as worker, \
                    mock.patch.object(qa_process, "_quarantine", object()):
                with self.assertRaisesRegex(runtime.RuntimeError, "cleanup remains unresolved") as failure:
                    runtime.prepare_runtime(self.destination, "x64")
                self.assertNotIn("private", str(failure.exception))
                self.assertEqual(len(stages), 1)
                self.assertEqual((stages[0] / "runtime.nupkg").read_bytes(), b"unfinished fixture")
                self.assertFalse((self.destination / "conpty.dll").exists())
                self.assertFalse((self.destination / "x64/OpenConsole.exe").exists())
                lease_file = runtime.dev_cache.leases_root(root=runtime.ROOT) / "windows-conpty-runtime.lock"
                with self.assertRaises(OSError):
                    with runtime.dev_cache._locked_lease_file(lease_file, create=False, nonblocking=True):
                        self.fail("unresolved worker lost its cache lease")
                with mock.patch.object(runtime.dev_cache, "cache_lease") as lease:
                    for _ in range(2):
                        with self.assertRaisesRegex(runtime.RuntimeError, "cleanup remains unresolved"):
                            self.prepare()
                    lease.assert_not_called()
                self.assertEqual(worker.call_count, 1)
                self.assertEqual(list(stages[0].parent.glob("conpty-*")), stages)
        finally:
            # The fixture has no native worker; release only its simulated quarantine.
            quarantine = getattr(runtime, "_preparation_quarantine", None)
            if quarantine is not None:
                quarantine[1].__exit__(None, None, None)
                runtime._preparation_quarantine = None

    def test_linked_destination_parent_is_rejected(self):
        actual = self.root / "actual"
        actual.mkdir()
        linked = self.root / "linked"
        try:
            linked.symlink_to(actual, target_is_directory=True)
        except OSError:
            self.skipTest("native symlink creation is not permitted")
        with self.assertRaisesRegex(runtime.RuntimeError, "links or reparse"):
            runtime.prepare_runtime(linked / "child", "x64", offline=True, archive=self.archive)
        self.assertFalse((actual / "child").exists())


class DownloadDeadlineTests(unittest.TestCase):
    def test_header_and_chunk_trickle_have_an_owned_wall_clock_deadline(self):
        # Exercise real urllib blocking reads against an owned loopback peer.
        # The outer QA owner is the test watchdog; it also bounds the old bug.
        for phase in ("headers", "chunked"):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as temporary:
                stop = threading.Event()
                connected = threading.Event()
                listener = socket.socket()
                listener.bind(("127.0.0.1", 0))
                listener.listen(1)
                listener.settimeout(0.2)
                url = f"http://127.0.0.1:{listener.getsockname()[1]}/fixture"
                def serve():
                    try:
                        while not stop.is_set():
                            try:
                                connection, _ = listener.accept()
                                connected.set()
                                break
                            except socket.timeout:
                                continue
                        else:
                            return
                        with connection:
                            connection.settimeout(0.2)
                            connection.recv(8192)
                            prefix = (b"HTTP/1.1 200 OK\r\nX-Progress: " if phase == "headers"
                                      else b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n800000\r\n")
                            connection.sendall(prefix)
                            while not stop.wait(0.02):
                                connection.sendall(b"x")
                    except OSError:
                        pass  # The deadline closes the client before the fixture stops.
                server = threading.Thread(target=serve, name="conpty-fixture-peer", daemon=True)
                server.start()
                script = r'''
import json, pathlib, sys, threading, time
import windows_conpty_runtime as runtime
import qa_process
runtime.DOWNLOAD_SECONDS = 0.8
runtime.load_recipe = lambda: {"package": {"url": sys.argv[1]}}
run = qa_process.run
def fixture_worker(command, **kwargs):
    # Only the test substitutes its loopback peer; production worker CLI has no URL option.
    assert command[:3] == [sys.executable, str(pathlib.Path(runtime.__file__).resolve()), "--download-worker"]
    command = [sys.executable, "-c",
        "import pathlib,sys; import windows_conpty_runtime as r; r._download_stream(sys.argv[1], pathlib.Path(sys.argv[2]))",
        sys.argv[1], command[-1]]
    kwargs["cwd"] = pathlib.Path(runtime.__file__).parent
    return run(command, **kwargs)
runtime.qa_process = qa_process
qa_process.run = fixture_worker
started = time.monotonic()
try:
    runtime._download(sys.argv[1], pathlib.Path(sys.argv[2]))
except runtime.RuntimeError:
    print(json.dumps({"elapsed": time.monotonic()-started,
        "quarantined": qa_process._quarantine is not None,
        "locked": qa_process._launch_lock.locked(),
        "readers": any(t.name.startswith("automexia-qa-") for t in threading.enumerate())}))
else:
    raise AssertionError("trickling download unexpectedly completed")
'''
                chunks = []
                try:
                    result = qa_process.run(
                        [sys.executable, "-c", script, url, str(Path(temporary) / "runtime.nupkg")],
                        cwd=Path(runtime.__file__).parent, timeout_seconds=5,
                        consume=chunks.append,
                    )
                finally:
                    stop.set()
                    listener.close()
                    server.join(timeout=2)
                self.assertFalse(server.is_alive(), "fixture peer did not retire")
                self.assertTrue(connected.is_set(), "download never reached the fixture peer")
                self.assertFalse(result.timed_out, "download escaped its own deadline into the outer watchdog")
                self.assertEqual(result.return_code, 0, b"".join(chunks).decode(errors="replace"))
                evidence = json.loads(b"".join(chunks))
                self.assertLess(evidence["elapsed"], 3)
                self.assertGreaterEqual(evidence["elapsed"], 0.7)
                self.assertFalse(evidence["quarantined"] or evidence["locked"] or evidence["readers"])


class RecipeAndPeTests(unittest.TestCase):
    def test_local_build_copy_and_native_gui_share_runtime_preparation(self):
        source = (runtime.ROOT / "tools/xtask/src/main.rs").read_text(encoding="utf-8")
        for start, end, before in (
            ("fn build_debug_app()", "fn prepare_windows_runtime(", "    Ok(())"),
            ("fn stage_runtime_binary(", "fn generation_binary_name(", "fs::copy(build_binary"),
            ("fn test_resize_stress(", "    let report_directory", "    let report_directory"),
        ):
            with self.subTest(owner=start):
                begin = source.index(start)
                finish = source.index(end, begin) + len(end)
                body = source[begin:finish]
                self.assertIn("prepare_windows_runtime(", body)
                self.assertLess(body.index("prepare_windows_runtime("), body.index(before))

    def test_real_recipe_is_pinned_and_maps_architectures(self):
        recipe = runtime.load_recipe()
        self.assertEqual(runtime.runtime_version(), "1.24.260710001")
        self.assertEqual(recipe["package"]["sha256"], "175640566a3b59c4b132070ee96c2c77e5ab7edd2e92732a5eb3610bbf63d90e")
        self.assertEqual(runtime.runtime_files("x64"), runtime.runtime_files("x86_64-pc-windows-msvc"))
        self.assertEqual(runtime.runtime_files("arm64"), runtime.runtime_files("aarch64-pc-windows-msvc"))
        with self.assertRaises(runtime.RuntimeError):
            runtime.runtime_files("x86")

    def test_pe_reader_rejects_malformed_and_unbounded_offsets(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.exe"
            for data in (b"", b"MZ", pe(0x14C), b"MZ" + bytes(58) + b"\xff" * 4):
                path.write_bytes(data)
                with self.assertRaises(runtime.RuntimeError):
                    runtime.pe_architecture(path)

    def test_bad_recipe_is_rejected(self):
        recipe = runtime.load_recipe()
        for mutate in (lambda r: r.update(schema_version=2),
                       lambda r: r["package"].update(url="http://untrusted.invalid/package"),
                       lambda r: r["package"].update(publisher="Other Publisher"),
                       lambda r: r["package"].update(license="Unknown"),
                       lambda r: r["files"][0].update(path="../conpty.dll"),
                       lambda r: r["files"].pop()):
            altered = copy.deepcopy(recipe)
            mutate(altered)
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "recipe.json"
                path.write_text(json.dumps(altered))
                with mock.patch.object(runtime, "RECIPE", path), self.assertRaises(runtime.RuntimeError):
                    runtime.load_recipe()


if __name__ == "__main__":
    unittest.main()
