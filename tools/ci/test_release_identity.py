"""Release identity and real plist serialization regressions."""
import plistlib
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import release_identity as identity

ROOT = Path(__file__).resolve().parents[2]


class ReleaseIdentityTests(unittest.TestCase):
    def test_debian_changelog_starts_with_current_workspace_release(self):
        import tomllib
        version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
        header = (ROOT / "packaging/linux/changelog.Debian").read_text().splitlines()[0]
        self.assertEqual(header, f"automexia-terminal ({version.replace('-', '~')}-1) unstable; urgency=medium")

    def test_alpha_tag_cargo_and_native_versions_are_distinct(self):
        self.assertEqual(identity.release_tag("0.0.1-alpha"), "0.0.1_alpha")
        self.assertEqual(identity.native_version("0.0.1-alpha"), "0.0.1")
        self.assertEqual(identity.release_tag("0.0.2-alpha"), "v0.0.2-alpha")
        self.assertEqual(identity.native_version("0.0.2-alpha"), "0.0.2")
        self.assertEqual(identity.release_tag("0.0.3-alpha"), "v0.0.3-alpha")
        self.assertEqual(identity.native_version("0.0.3-alpha"), "0.0.3")
        self.assertEqual(identity.release_tag("0.0.4-alpha"), "v0.0.4-alpha")
        self.assertEqual(identity.native_version("0.0.4-alpha"), "0.0.4")
        self.assertEqual(identity.release_tag("1.2.3"), "v1.2.3")
        self.assertEqual(identity.native_version("255.255.65535"), "255.255.65535")

    def test_application_release_preserves_the_maintained_library_identities(self):
        import tomllib
        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
        packages = {item["name"]: item for item in lock if "source" not in item}
        self.assertEqual(packages["automexia-terminal"]["version"], workspace["package"]["version"])
        for name in ("corcovado", "rio-backend", "rio-fonts", "rio-grapheme-width",
                     "rio-graphics", "rio-notifier", "rio-vt", "rio-window",
                     "sugarloaf", "teletypewriter"):
            with self.subTest(library=name):
                package = tomllib.loads((ROOT / name / "Cargo.toml").read_text())["package"]
                self.assertEqual(package["version"], "0.4.0")
                self.assertEqual(workspace["dependencies"][name]["version"], package["version"])
                self.assertEqual(packages[name]["version"], package["version"])

    def test_direct_library_consumers_keep_compatible_versions(self):
        import tomllib
        embedding = tomllib.loads((ROOT / "librio/Cargo.toml").read_text())
        native = tomllib.loads((ROOT / "third-party/accesskit-windows/Cargo.toml").read_text())
        for name, dependency in (("rio-vt", embedding["dependencies"]["rio-vt"]),
                                 ("rio-window", native["target"]["cfg(windows)"]["dev-dependencies"]["winit"])):
            package = tomllib.loads((ROOT / name / "Cargo.toml").read_text())["package"]
            self.assertEqual(dependency["version"], package["version"])

    def test_fuzz_lockfile_tracks_application_identity_and_reviewed_dependencies(self):
        import tomllib
        runtime = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
        fuzz = tomllib.loads((ROOT / "fuzz/Cargo.lock").read_text())["package"]
        local = {item["name"]: item for item in runtime if "source" not in item}
        fuzz_local = {item["name"]: item for item in fuzz if "source" not in item}
        shared = local.keys() & fuzz_local.keys()
        self.assertIn("automexia-terminal", shared)
        for name in shared:
            with self.subTest(package=name):
                self.assertEqual(fuzz_local[name]["version"], local[name]["version"])
        for name in ("zip", "ctutils", "clipboard-win"):
            with self.subTest(dependency=name):
                reviewed = {(item["version"], item["checksum"])
                            for item in runtime if item["name"] == name}
                used = {(item["version"], item["checksum"])
                        for item in fuzz if item["name"] == name}
                self.assertTrue(reviewed)
                self.assertEqual(used, reviewed)

    def test_rejects_ambiguous_injected_and_unrepresentable_versions(self):
        for value in ("0.0.1_alpha", "v0.0.1-alpha", "01.2.3", "1.2.3\n", "1.2.3;id",
                      "1.2", "1.2.3.4", "1.2.3+dev", "1.2.3-beta", "256.1.1", "1.256.1", "1.2.65536"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                identity.release_tag(value)

    def test_macos_plist_preserves_resources_and_uses_native_numeric_fields(self):
        template = ROOT / "packaging/macos/Info.plist"
        original = plistlib.loads(template.read_bytes())
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "Info.plist"
            for version in ("0.0.1-alpha", "1.2.3"):
                identity.render_macos_plist(template, out, version)
                actual = plistlib.loads(out.read_bytes())
                self.assertEqual(actual["CFBundleShortVersionString"], identity.native_version(version))
                self.assertEqual(actual["CFBundleVersion"], identity.native_version(version))
                self.assertEqual(actual["AutomexiaReleaseVersion"], version)
                for field, value in original.items():
                    if field not in ("CFBundleShortVersionString", "CFBundleVersion"):
                        self.assertEqual(actual[field], value)

    def test_alpha_source_policy_is_exact_and_external_publisher_is_inert(self):
        import re
        import stable_release
        import yaml
        policy = stable_release.load_policy()
        self.assertEqual(policy["repository"], "AmjedAllaya/automexia")
        pattern = policy["release_source"]["tag_pattern"]
        self.assertIsNotNone(re.fullmatch(pattern, "0.0.1_alpha"))
        for wrong in ("0.0.2_alpha", "0.0.1_alpha/other", "0.0.1_alpha\n"):
            self.assertIsNone(re.fullmatch(pattern, wrong))
        legacy = yaml.safe_load((ROOT / ".github/workflows/linux-early-access.yml").read_text())
        self.assertTrue(legacy["jobs"]["authorize"]["if"].startswith("false && ("))
        release = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())["jobs"]
        gate = release["release-final-gate"]
        for required in ("verify-linux-final", "verify-windows-final", "verify-macos-final", "native-platform-qa"):
            self.assertIn(required, gate["needs"])
        self.assertEqual(release["native-platform-qa"]["uses"], "./.github/workflows/accessibility-native.yml")
        publication = release["publish-release"]["steps"][-1]["run"]
        self.assertIn('--prerelease --latest=false', publication)
        self.assertNotIn('automexia-releases', publication)
        self.assertIn('gh release verify-asset', publication)

    def test_release_candidate_gate_accepts_only_the_canonical_alpha_and_stable_versions(self):
        import os
        import yaml
        if sys.platform == "win32":
            self.skipTest("release authorization Bash executes on the Ubuntu runner")
        jobs = yaml.safe_load((ROOT / ".github/workflows/ci.yml").read_text())["jobs"]
        step = next(item for item in jobs["release-candidate"]["steps"]
                    if item.get("name", "").startswith("Validate internal release branch"))
        guard = step["run"].split("rustup toolchain install", 1)[0]
        for branch, success in (("release/0.0.1-alpha", True), ("release/1.2.3", True),
                                ("release/0.0.2-alpha", True), ("release/0.0.3-alpha", True),
                                ("release/0.0.4-alpha", True), ("release/0.0.5-alpha", False),
                                ("release/0.0.1_alpha", False),
                                ("release/01.2.3", False), ("release/1.2.3+test", False)):
            with self.subTest(branch=branch):
                result = subprocess.run(["bash", "-c", guard], cwd=ROOT, capture_output=True,
                                        env={**os.environ, "HEAD_REPO": "fixture/repository",
                                             "GITHUB_REPOSITORY": "fixture/repository", "HEAD_BRANCH": branch},
                                        timeout=10)
                self.assertEqual(result.returncode == 0, success, result.stderr)
        self.assertIn('"refs/tags/$tag"', step["run"])

    def test_publication_fetches_created_tag_before_local_provenance_validation(self):
        import yaml
        jobs = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())["jobs"]
        script = jobs["publish-release"]["steps"][-1]["run"]
        fetch = 'git fetch --no-tags origin "refs/tags/$tag:refs/tags/$tag"'
        self.assertIn(fetch, script)
        self.assertLess(script.index('gh api --method POST "repos/$GITHUB_REPOSITORY/git/refs"'), script.index(fetch))
        self.assertLess(script.index(fetch), script.index('stable_release.py validate-source'))
        self.assertLess(script.index('stable_release.py validate-source'), script.index('gh release create'))

    def test_archives_are_deterministic_and_preserve_payloads(self):
        import os
        import tarfile
        import zipfile
        import reproducible_archive as archive
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            (source / "terminal").write_bytes(b"executable bytes")
            (source / "terminal").chmod(0o755)
            (source / "nested").mkdir()
            (source / "nested/README").write_bytes(b"documentation")
            for kind in ("zip", "tar.gz"):
                out = root / ("portable." + kind)
                archive.build(source, out, kind, 1700000000)
                before = out.read_bytes()
                os.utime(source / "terminal", (1800000000, 1800000000))
                archive.build(source, out, kind, 1700000000)
                self.assertEqual(out.read_bytes(), before)
                if kind == "zip":
                    with zipfile.ZipFile(out) as package:
                        self.assertEqual(package.namelist(), ["nested/README", "terminal"])
                        self.assertEqual(package.read("terminal"), b"executable bytes")
                else:
                    with tarfile.open(out) as package:
                        self.assertEqual(package.getnames(), ["nested/README", "terminal"])
                        self.assertEqual(package.extractfile("terminal").read(), b"executable bytes")
                        self.assertEqual(package.getmember("terminal").uid, 0)

    def test_archive_failure_preserves_previous_output(self):
        from unittest import mock
        import reproducible_archive as archive
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            payload = source / "large"
            payload.write_bytes(b"too much")
            output = root / "previous.zip"
            output.write_bytes(b"previous valid archive")
            with mock.patch.object(archive, "MAX_FILE_BYTES", 3), self.assertRaises(ValueError):
                archive.build(source, output, "zip", 1700000000)
            self.assertEqual(output.read_bytes(), b"previous valid archive")
            self.assertEqual(list(root.glob(".archive-*")), [])
            with self.assertRaises(ValueError):
                archive.build(source, source / "inside.zip", "zip", 1700000000)

    def test_archive_refuses_links(self):
        import os
        import reproducible_archive as archive
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            (source / "file").write_bytes(b"data")
            try:
                os.link(source / "file", source / "alias")
            except OSError:
                self.skipTest("hard links unavailable")
            with self.assertRaisesRegex(ValueError, "unlinked"):
                archive.build(source, root / "out.zip", "zip", 1700000000)

    def test_every_native_release_target_has_required_reproducibility_evidence(self):
        import yaml
        jobs = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())["jobs"]
        build_targets = {item["target"] for item in jobs["build"]["strategy"]["matrix"]["include"]}
        covered = set()
        for name in ("reproducibility-linux", "reproducibility-desktop"):
            self.assertIn(name, jobs["release-final-gate"]["needs"])
            job = jobs[name]
            covered.update(item["target"] for item in job["strategy"]["matrix"]["include"])
            self.assertFalse(job.get("continue-on-error", False))
        self.assertEqual(covered, build_targets)

    def test_reproducibility_compares_every_shipped_runtime_and_rejects_missing_files(self):
        import reproducible_build as build
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            left, right = root / "left", root / "right"
            left.mkdir(); right.mkdir()
            for target in sorted(build.TARGETS):
                suffix = ".exe" if "windows" in target else ""
                for runtime in build.RUNTIMES:
                    for folder in (left, right):
                        (folder / (runtime + suffix)).write_bytes(b"identical payload")
                self.assertEqual(len(build.compare_binaries(left, right, target)), 4)
                for runtime in build.RUNTIMES:
                    changed = right / (runtime + suffix)
                    changed.write_bytes(b"different payload")
                    with self.assertRaisesRegex(ValueError, "non-reproducible"):
                        build.compare_binaries(left, right, target)
                    changed.unlink()
                    with self.assertRaisesRegex(ValueError, "missing regular"):
                        build.compare_binaries(left, right, target)
                    changed.write_bytes(b"identical payload")

    def test_reproducibility_does_not_accept_cached_file_comparisons(self):
        import os
        import reproducible_build as build
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            left, right = root / "left", root / "right"
            left.mkdir(); right.mkdir()
            for runtime in build.RUNTIMES:
                for folder in (left, right):
                    (folder / runtime).write_bytes(b"original")
            build.compare_binaries(left, right, "x86_64-unknown-linux-gnu")
            changed = right / "amx"
            before = changed.stat()
            changed.write_bytes(b"modified")
            os.utime(changed, ns=(before.st_atime_ns, before.st_mtime_ns))
            with self.assertRaisesRegex(ValueError, "non-reproducible"):
                build.compare_binaries(left, right, "x86_64-unknown-linux-gnu")

    @unittest.skipUnless(sys.platform == "win32", "requires native Windows resource compiler")
    def test_native_windows_resources_rebuild_identically(self):
        import shutil
        import reproducible_build as build
        host = next(line[6:] for line in subprocess.check_output(
            ["rustc", "-vV"], text=True, timeout=30).splitlines() if line.startswith("host: "))
        self.assertIn(host, ("x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"))
        outputs = []
        with tempfile.TemporaryDirectory(prefix="automexia-resource-repro-") as directory:
            for attempt in (1, 2):
                root = Path(directory) / str(attempt)
                package = root / "apps/automexia-terminal"
                (package / "src/bin").mkdir(parents=True)
                for relative in ("assets/brand/automexia-terminal.ico", "packaging/windows/automexia.manifest"):
                    destination = root / relative
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(ROOT / relative, destination)
                shutil.copyfile(ROOT / "apps/automexia-terminal/build.rs", package / "build.rs")
                (package / "Cargo.toml").write_text(
                    '[package]\nname="automexia-terminal"\nversion="0.0.1-alpha"\nedition="2021"\n'
                    '[workspace]\n[build-dependencies]\nwinres="=0.1.12"\n'
                    '[profile.release]\nstrip="symbols"\ndebug="full"\nsplit-debuginfo="packed"\n'
                    'lto=true\npanic="abort"\ncodegen-units=1\nincremental=false\n', encoding="utf-8")
                (package / "src/lib.rs").write_text('pub fn marker() -> u32 { 7 }\n', encoding="utf-8")
                for runtime in build.RUNTIMES:
                    source = 'fn main() { println!("{}", automexia_terminal::marker()); }\n'
                    if runtime == "amx":
                        source = 'fn main() { println!("launcher"); }\n'
                    (package / "src/bin" / (runtime + ".rs")).write_text(source, encoding="utf-8")
                env = build.build_environment(root, host, 1700000000)
                for argv in (["cargo", "generate-lockfile", "--offline"],
                             ["cargo", "build", "--offline", "--locked", "--release", "--target", host]):
                    result = subprocess.run(argv, cwd=package, env=env, capture_output=True,
                                            text=True, timeout=180)
                    self.assertEqual(result.returncode, 0, "native resource fixture build failed")
                outputs.append(root / "target" / host / "release")
            self.assertEqual(len(build.compare_binaries(*outputs, host)), len(build.RUNTIMES))

    @unittest.skipUnless(sys.platform in ("darwin", "linux"), "requires native Unix symlink semantics")
    def test_native_unix_symlinked_source_rebuilds_identically(self):
        import reproducible_build as build
        host = next(line[6:] for line in subprocess.check_output(
            ["rustc", "-vV"], text=True, timeout=30).splitlines() if line.startswith("host: "))
        self.assertIn(host, build.TARGETS)
        outputs = []
        with tempfile.TemporaryDirectory(prefix="automexia-path-repro-") as directory:
            # Packaged inputs originate in a direct checkout; verification also
            # uses temporary source archives behind the macOS /var symlink.
            # Vary both path form and length, including a non-ASCII checkout.
            for attempt, name in enumerate(("checkout", "archive-with-a-longer-name", "source-\u00e9")):
                real = Path(directory).resolve() / name
                root = real if attempt == 0 else Path(directory) / f"logical-{attempt}"
                (real / "src/bin").mkdir(parents=True)
                if attempt:
                    root.symlink_to(real, target_is_directory=True)
                (root / "Cargo.toml").write_text(
                    '[package]\nname="repro-fixture"\nversion="0.1.0"\nedition="2021"\n[workspace]\n'
                    '[profile.release]\nstrip="symbols"\ndebug="full"\nsplit-debuginfo="packed"\n'
                    'lto=true\npanic="abort"\ncodegen-units=1\nincremental=false\n', encoding="utf-8")
                (root / "src/payload.rs").write_text('fn main() { println!("{}", file!()); }\n', encoding="utf-8")
                for runtime in build.RUNTIMES:
                    (root / "src/bin" / (runtime + ".rs")).write_text(
                        'include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/payload.rs"));\n', encoding="utf-8")
                result = subprocess.run(["cargo", "build", "--offline", "--release", "--target", host],
                    cwd=root, env=build.build_environment(root, host, 1700000000),
                    capture_output=True, text=True, timeout=180)
                self.assertEqual(result.returncode, 0, "native path-remapping fixture build failed")
                outputs.append(root / "target" / host / "release")
            for output in outputs[1:]:
                for runtime in build.RUNTIMES:
                    before = (outputs[0] / runtime).read_bytes()
                    after = (output / runtime).read_bytes()
                    # Retain bounded, content-free native failure diagnostics.
                    changed = [i for i, pair in enumerate(zip(before, after)) if pair[0] != pair[1]]
                    detail = {"runtime": runtime, "sizes": [len(before), len(after)],
                              "changed_bytes": len(changed), "first_offsets": changed[:20],
                              "last_offsets": changed[-20:]}
                    self.assertTrue(before == after, str(detail))
                self.assertEqual(len(build.compare_binaries(outputs[0], output, host)), len(build.RUNTIMES))

    def test_reproducibility_diagnostics_identify_stage_without_binary_content_or_paths(self):
        import contextlib
        import io
        from unittest import mock
        import reproducible_build as build
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            left, right = root / "private-left", root / "private-right"
            left.mkdir(); right.mkdir()
            (left / "automexia.exe").write_bytes(b"same-private-canary")
            (right / "automexia.exe").write_bytes(b"same-PRIVATE-canary")
            for stage in ("cold-builds", "packaged-inputs"):
                with self.assertRaises(build.RuntimeComparisonError) as raised:
                    build.compare_binaries(left, right, "x86_64-pc-windows-msvc", stage=stage)
                error = raised.exception
                stream = io.StringIO()
                with mock.patch.object(sys, "argv", ["repro", "report.json", "--target", "x86_64-pc-windows-msvc"]), \
                        mock.patch.object(build, "run", side_effect=error), contextlib.redirect_stderr(stream), \
                        self.assertRaises(SystemExit) as exited:
                    build.main()
                self.assertEqual(exited.exception.code, 1)
                self.assertIn("stage=" + stage, stream.getvalue())
                self.assertIn("first differing byte=5", stream.getvalue())
                self.assertNotIn("private", stream.getvalue())
                self.assertNotIn(str(root), stream.getvalue())
            for runtime, reason, stage, offset in (("private-name", "different", "comparison", 0),
                    ("automexia", "private-reason", "comparison", 0),
                    ("automexia", "different", "private-stage", 0),
                    ("automexia", "different", "comparison", -1),
                    ("automexia", "different", "comparison", True)):
                with self.assertRaisesRegex(ValueError, "invalid comparison diagnostic"):
                    build.RuntimeComparisonError(runtime, reason, stage, offset)

    def test_windows_release_jobs_run_native_resource_regression(self):
        import yaml
        for name in ("first-alpha.yml", "release.yml"):
            document = yaml.safe_load((ROOT / ".github/workflows" / name).read_text(encoding="utf-8"))
            steps = document["jobs"]["build"]["steps"]
            checks = [step for step in steps if "-k native_windows_resources" in step.get("run", "")]
            self.assertEqual(len(checks), 1)
            self.assertEqual(checks[0]["if"], "runner.os == 'Windows'")
            self.assertFalse(checks[0].get("continue-on-error", False))

    def test_reproducibility_is_cold_native_and_preserves_target_flags(self):
        import reproducible_build as build
        from unittest import mock
        with tempfile.TemporaryDirectory() as directory:
            source = (Path(directory) / "source").resolve()
            for target in build.TARGETS:
                environment = build.build_environment(source, target, 1700000000)
                self.assertEqual(environment["CARGO_INCREMENTAL"], "0")
                self.assertEqual(environment["SOURCE_DATE_EPOCH"], "1700000000")
                self.assertTrue(Path(environment["CARGO_TARGET_DIR"]).is_relative_to(source))
                self.assertTrue(Path(environment["CARGO_BUILD_BUILD_DIR"]).is_relative_to(source))
                if "windows" in target:
                    self.assertIn("windows_slim_errors", environment["CARGO_ENCODED_RUSTFLAGS"])
                    self.assertIn("/Brepro", environment["CARGO_ENCODED_RUSTFLAGS"])
                    self.assertIn("-Ctarget-feature=+crt-static", environment["CARGO_ENCODED_RUSTFLAGS"].split("\x1f"))
                else:
                    self.assertNotIn("crt-static", environment["CARGO_ENCODED_RUSTFLAGS"])
                if "apple" in target:
                    self.assertIn("-Clink-arg=-Wl,-oso_prefix,.", environment["CARGO_ENCODED_RUSTFLAGS"])
                else:
                    self.assertNotIn("oso_prefix", environment["CARGO_ENCODED_RUSTFLAGS"])
                self.assertNotIn("no_uuid", environment["CARGO_ENCODED_RUSTFLAGS"])
                if target == "aarch64-apple-darwin":
                    self.assertIn("apple-m1", environment["CARGO_ENCODED_RUSTFLAGS"])
            with mock.patch.object(build, "capture", return_value="host: x86_64-unknown-linux-gnu"), self.assertRaisesRegex(ValueError, "native host"):
                build.run(Path(directory), "aarch64-apple-darwin", Path(directory) / "result.json")
            self.assertFalse((Path(directory) / "result.json").exists())

    def test_windows_crt_policy_survives_reproducible_flag_override(self):
        import reproducible_build as build
        import tomllib
        from unittest import mock
        config = tomllib.loads((ROOT / ".cargo/config.toml").read_text(encoding="utf-8"))
        flags = config["target"]['cfg(all(target_os = "windows", target_env = "msvc"))']["rustflags"]
        self.assertIn("-Ctarget-feature=+crt-static", flags)
        for target in ("x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"):
            with mock.patch.dict(build.os.environ, {"RUSTFLAGS": "-Ctarget-feature=-crt-static",
                                                    "CARGO_ENCODED_RUSTFLAGS": "-Ctarget-feature=-crt-static"}):
                environment = build.build_environment(ROOT, target, 1700000000)
            self.assertNotIn("RUSTFLAGS", environment)
            flags = environment["CARGO_ENCODED_RUSTFLAGS"].split("\x1f")
            self.assertIn("-Ctarget-feature=+crt-static", flags)
            self.assertNotIn("-Ctarget-feature=-crt-static", flags)

    def test_packaging_does_not_pass_flags_rejected_by_the_pinned_packager(self):
        for name in ("tools/xtask/src/main.rs", ".github/workflows/nightly.yml"):
            self.assertNotIn("--no-build", (ROOT / name).read_text(), name)

    def test_native_package_adapter_preserves_metadata_and_uses_supported_exact_arguments(self):
        import json
        import tomllib
        from unittest import mock
        with tempfile.TemporaryDirectory() as directory:
            binaries = Path(directory) / "release inputs"
            output = Path(directory) / "native packages"
            original = tomllib.loads((ROOT / "apps/automexia-terminal/Cargo.toml").read_text())["package"]["metadata"]["packager"]
            config = identity.native_packager_config(ROOT, "0.0.1-alpha", binaries, output)
            self.assertEqual(config["version"], "0.0.1")
            self.assertEqual(config["binaries-dir"], str(binaries.resolve()))
            self.assertEqual(config["out-dir"], str(output.resolve()))
            for field, value in original.items():
                self.assertEqual(config[field], value, field)
            for target, formats in (("x86_64-pc-windows-msvc", "wix"), ("aarch64-apple-darwin", "app,dmg")):
                with mock.patch.object(identity.subprocess, "run") as run:
                    identity.package_native(ROOT, "0.0.1-alpha", binaries, output, target, formats)
                    argv = run.call_args.args[0]
                    self.assertEqual(argv[:3], ["cargo", "packager", "--config"])
                    self.assertEqual(json.loads(argv[3]), config)
                    self.assertEqual(argv[4:], ["--target", target, "--formats", formats])
                    self.assertEqual(run.call_args.kwargs["cwd"], ROOT / "apps/automexia-terminal")
                    self.assertTrue(run.call_args.kwargs["check"])
                    self.assertEqual(run.call_args.kwargs["env"]["CARGO_PACKAGER_AUTOMEXIA_CONPTY_ROOT"], str(binaries.resolve()))
            with mock.patch.object(identity.subprocess, "run") as run, self.assertRaises(ValueError):
                identity.package_native(ROOT, "0.0.1-alpha", binaries, output, "x86_64-pc-windows-msvc", "app,dmg")
            run.assert_not_called()

    def test_native_package_vendor_root_cannot_be_overridden_by_inherited_environment(self):
        from unittest import mock
        with mock.patch.dict(identity.os.environ, {"CARGO_PACKAGER_AUTOMEXIA_CONPTY_ROOT": "unrelated inputs"}), mock.patch.object(identity.subprocess, "run") as run:
            inputs = ROOT / "target/reviewed inputs"
            identity.package_native(ROOT, "0.0.1-alpha", inputs, ROOT / "target/package", "x86_64-pc-windows-msvc", "wix")
            self.assertEqual(run.call_args.kwargs["env"]["CARGO_PACKAGER_AUTOMEXIA_CONPTY_ROOT"], str(inputs.resolve()))

    def test_native_package_adapter_rejects_hooks_that_can_change_reviewed_inputs(self):
        import shutil
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "apps/automexia-terminal/Cargo.toml"
            manifest.parent.mkdir(parents=True)
            shutil.copyfile(ROOT / "apps/automexia-terminal/Cargo.toml", manifest)
            source = manifest.read_text().replace('[package.metadata.packager]',
                    '[package.metadata.packager]\nbefore-packaging-command = "unreviewed-build"')
            manifest.write_text(source)
            with self.assertRaisesRegex(ValueError, "build hooks"):
                identity.native_packager_config(root, "0.0.1-alpha", root / "binaries", root / "packages")

    def test_release_fragments_can_be_consumed_by_the_existing_assembler(self):
        categories = {"Added", "Changed", "Fixed", "Security", "Deprecated", "Removed"}
        for path in (ROOT / "changes").glob("*.md"):
            if path.name == "README.md":
                continue
            with self.subTest(fragment=path.name):
                heading, _, body = path.read_text().partition("\n")
                self.assertIn(heading.strip(), categories)
                self.assertTrue(body.strip().startswith("- "))

    def test_cli_rejects_invalid_before_writing(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "Info.plist"
            out.write_bytes(b"unchanged")
            result = subprocess.run([sys.executable, str(ROOT / "tools/ci/release_identity.py"),
                                     "plist", "invalid", "--template", str(ROOT / "packaging/macos/Info.plist"),
                                     "--output", str(out)], capture_output=True, timeout=10)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(out.read_bytes(), b"unchanged")


if __name__ == "__main__":
    unittest.main(verbosity=2)
