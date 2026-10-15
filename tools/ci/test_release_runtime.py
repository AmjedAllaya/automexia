#!/usr/bin/env python3
"""Exercise release runtime staging without building, signing, or installing."""

from __future__ import annotations

import os
import json
import plistlib
import gzip
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest
import zipfile

import yaml


ROOT = Path(__file__).resolve().parents[2]
RUNTIMES = ("automexia", "amx", "automexia-suggestion-helper", "automexia-ssh-helper")
SHELL_ASSETS = (
    "bash/automexia.bash", "cmd/automexia-ls.cmd", "cmd/automexia-ls.ps1", "cmd/automexia.cmd",
    "completion/bash/automexia-completion.bash", "completion/fish/automexia-completion.fish",
    "completion/powershell/automexia-completion.ps1", "completion/zsh/automexia-completion.zsh",
    "fish/automexia.fish", "install-unix.sh", "install-windows.ps1", "posix/automexia-eza-filter.pl", "posix/automexia-os.sh",
    "powershell/automexia.format.ps1xml", "powershell/automexia.ps1", "uninstall-unix.sh",
    "uninstall-windows.ps1", "windows-path-safety.ps1", "windows-wsl.ps1", "zsh/automexia.zsh",
)


def workflow_step(workflow: str, name: str) -> dict:
    document = yaml.safe_load((ROOT / ".github/workflows" / workflow).read_text(encoding="utf-8"))
    matches = [step for job in document["jobs"].values() for step in job.get("steps", [])
               if step.get("name") == name]
    if len(matches) != 1:
        raise AssertionError(f"expected one workflow step named {name!r}")
    return matches[0]


def bash_program() -> str | None:
    # Windows' system bash.exe is WSL, not a shell for the Windows fixture tree.
    if os.name == "nt":
        candidate = Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Git/bin/bash.exe"
        return str(candidate) if candidate.is_file() else None
    return shutil.which("bash")


def write_fixture_script(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("#!/usr/bin/env bash\nset -euo pipefail\n" + body, encoding="utf-8", newline="\n")
    path.chmod(0o755)


def runtime_fixture(runtime: str) -> str:
    if runtime in ("automexia-suggestion-helper", "automexia-ssh-helper"):
        return "echo 'helper must not be launched by package assembly' >&2\nexit 93\n"
    return f'''printf '%s\\n' '{runtime}' >> "$FIXTURE_TRACE/cli"
[[ "$#" -eq 1 && "$1" == --version ]]
printf '%s\\n' '0.4.0'
'''


class ReleaseRuntimeTests(unittest.TestCase):
    def run_fixture_step(self, root: Path, name: str, workflow: str = "release.yml") -> subprocess.CompletedProcess:
        bash = bash_program()
        if bash is None:
            self.skipTest("Bash unavailable for workflow package fixture")
        script = workflow_step(workflow, name)["run"]
        identity_tool = root / "tools/ci/release_identity.py"
        identity_tool.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / "tools/ci/release_identity.py", identity_tool)
        script = script.replace("${{ matrix.target }}", "fixture-target").replace("${{ matrix.artifact }}", "fixture-artifact")
        # The workflow executes unchanged apart from its matrix values. Only
        # external build/validation tools are replaced by bounded fixture scripts.
        script = 'export PATH="$PWD/fixture-bin:$PATH"\n' + script
        (root / "trace").mkdir()
        return subprocess.run([bash, "-c", script], cwd=root,
                              env={**os.environ, "FIXTURE_TRACE": "trace", "RUNNER_TEMP": "fixture-temp", "AUTOMEXIA_VERSION": "0.4.0"},
                              capture_output=True, text=True, timeout=30, check=False)

    def stage_fixture(self, platform: str, missing: str | None = None) -> tuple[subprocess.CompletedProcess, set[str]]:
        bash = bash_program()
        if bash is None:
            self.skipTest("Bash unavailable for workflow staging fixture")
        script = workflow_step("release.yml", "Stage portable artifact")["run"]
        script = script.replace("${{ matrix.target }}", "fixture-target").replace("${{ matrix.artifact }}", "fixture-artifact")
        suffix = ".exe" if platform == "Windows" else ""
        with tempfile.TemporaryDirectory(prefix="automexia-stage-") as directory:
            root = Path(directory)
            source = root / "target/fixture-target/release"
            source.mkdir(parents=True)
            for runtime in RUNTIMES:
                if runtime != missing:
                    (source / (runtime + suffix)).write_text(f"fixture {runtime}\n", encoding="utf-8")
            for name in ("LICENSE", "NOTICE.md", "THIRD_PARTY_NOTICES.md", "README.md"):
                (root / name).write_text("fixture\n", encoding="utf-8")
            result = subprocess.run([bash, "-c", script], cwd=root, env={**os.environ, "RUNNER_OS": platform},
                                    capture_output=True, text=True, timeout=30, check=False)
            staged = root / "dist/fixture-artifact"
            binaries = {path.name for path in staged.iterdir() if path.name.endswith(suffix)} if staged.exists() else set()
            if not suffix:
                binaries -= {"LICENSE", "NOTICE.md", "THIRD_PARTY_NOTICES.md", "README.md"}
            return result, binaries

    def test_linux_packages_ship_the_discoverable_complete_shell_resource_tree(self) -> None:
        manifest = yaml.safe_load((ROOT / "packaging/linux/nfpm.yaml").read_text())
        resources = [item for item in manifest["contents"]
                     if item["dst"] == "/usr/share/automexia/shell-integration"]
        self.assertEqual(len(resources), 1, "DEB/RPM must carry their session integration")
        resource = resources[0]
        self.assertEqual(resource.get("type"), "tree")
        source = ROOT / resource["src"]
        self.assertEqual(source.resolve(), (ROOT / "shell-integration").resolve())
        # Include hidden Zsh startup files; a shell wildcard omits them.
        for name in (*SHELL_ASSETS, "bash/session.bash", "bash/login-session.bash",
                     "zsh/session/.zshenv"):
            self.assertTrue((source / name).is_file(), name)

    def test_native_linux_package_payload_contains_session_resources(self) -> None:
        nfpm, dpkg, rpm, rpm2archive = (shutil.which(name) for name in ("nfpm", "dpkg-deb", "rpm", "rpm2archive"))
        if not all((nfpm, dpkg, rpm, rpm2archive)):
            self.skipTest("native DEB/RPM tools unavailable; manifest contract still runs")
        with tempfile.TemporaryDirectory(prefix="automexia-nfpm-") as directory:
            root = Path(directory)
            binary = root / "fixture-binary"
            binary.write_bytes(b"package fixture, not a release executable\n")
            compressed = root / "fixture.gz"
            compressed.write_bytes(gzip.compress(b"fixture\n", mtime=0))
            terminfo = root / "terminfo"
            terminfo.mkdir()
            (terminfo / "fixture").write_bytes(b"fixture")
            env = {**os.environ, "NFPM_ARCH": "amd64", "AUTOMEXIA_VERSION": "0.4.0",
                   "AUTOMEXIA_TERMINFO_ROOT": str(terminfo)}
            for key in ("AUTOMEXIA_BINARY", "AUTOMEXIA_CLI_BINARY",
                        "AUTOMEXIA_SUGGESTION_HELPER_BINARY", "AUTOMEXIA_SSH_HELPER_BINARY"):
                env[key] = str(binary)
            for key in ("AUTOMEXIA_MANPAGE", "AUTOMEXIA_CHANGELOG", "AUTOMEXIA_DEBIAN_CHANGELOG"):
                env[key] = str(compressed)
            for kind, listing, version in ((kind, listing, version)
                    for version in ("0.4.0", "0.0.1-alpha")
                    for kind, listing in (("deb", [dpkg, "--contents"]), ("rpm", [rpm, "-qpl"]))):
                env["AUTOMEXIA_VERSION"] = version
                env["SOURCE_DATE_EPOCH"] = "1700000000"
                package = root / ("fixture." + kind)
                result = subprocess.run([nfpm, "package", "--config", "packaging/linux/nfpm.yaml",
                                         "--packager", kind, "--target", str(package)],
                                        cwd=ROOT, env=env, capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 0, result.stderr)
                result = subprocess.run([*listing, str(package)], capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 0, result.stderr)
                for name in (*SHELL_ASSETS, "bash/session.bash", "bash/login-session.bash", "zsh/session/.zshenv"):
                    self.assertIn("/usr/share/automexia/shell-integration/" + name, result.stdout)
                query = [dpkg, "-f", str(package), "Version"] if kind == "deb" else [rpm, "-qp", "--queryformat", "%{VERSION}-%{RELEASE}", str(package)]
                actual_version = subprocess.check_output(query, text=True, timeout=30).strip()
                self.assertEqual(actual_version, version.replace("-", "~") + "-1")
                if version == "0.0.1-alpha":
                    self.assertIn("(" + actual_version + ")", (ROOT / "packaging/linux/changelog.Debian").read_text().splitlines()[0])
                before = package.read_bytes()
                result = subprocess.run([nfpm, "package", "--config", "packaging/linux/nfpm.yaml",
                                         "--packager", kind, "--target", str(package)],
                                        cwd=ROOT, env=env, capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(package.read_bytes(), before, "native package bytes must be reproducible")
                with tempfile.TemporaryDirectory(prefix="extracted-", dir=root) as extracted:
                    destination = Path(extracted)
                    if kind == "deb":
                        subprocess.run([dpkg, "--extract", str(package), str(destination)],
                                       check=True, capture_output=True, timeout=30)
                    else:
                        subprocess.run([rpm, "--checksig", "--nosignature", str(package)],
                                       check=True, capture_output=True, timeout=30)
                        payload = root / "payload.tar"
                        # rpm2cpio can return failure after emitting a complete
                        # payload on Ubuntu; use RPM's maintained archive converter.
                        with package.open("rb") as source, payload.open("wb") as output:
                            subprocess.run([rpm2archive, "-n", "-"], stdin=source, stdout=output,
                                           stderr=subprocess.PIPE, check=True, timeout=30)
                        with tarfile.open(payload) as archive:
                            archive.extractall(destination, filter="data")
                    for name in RUNTIMES:
                        self.assertEqual((destination / "usr/bin" / name).read_bytes(), binary.read_bytes())
                    for name in (*SHELL_ASSETS, "bash/session.bash", "zsh/session/.zshenv"):
                        self.assertEqual((destination / "usr/share/automexia/shell-integration" / name).read_bytes(),
                                         (ROOT / "shell-integration" / name).read_bytes())

    def test_portable_staging_preserves_all_runtime_binaries_on_every_os(self) -> None:
        for platform in ("Windows", "Linux", "macOS"):
            with self.subTest(platform=platform):
                result, binaries = self.stage_fixture(platform)
                self.assertEqual(result.returncode, 0, result.stderr)
                suffix = ".exe" if platform == "Windows" else ""
                self.assertEqual(binaries, {name + suffix for name in RUNTIMES})

    def test_portable_staging_rejects_each_missing_runtime(self) -> None:
        for platform in ("Windows", "Linux", "macOS"):
            for missing in RUNTIMES:
                with self.subTest(platform=platform, missing=missing):
                    result, _ = self.stage_fixture(platform, missing)
                    self.assertNotEqual(result.returncode, 0, "incomplete release runtime was accepted")

    def test_final_verifiers_download_only_their_own_platform_packages(self) -> None:
        jobs = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())["jobs"]
        for job, expected in (("verify-macos-final", "packages-macos-universal"),
                              ("verify-linux-final", "${{ matrix.artifact }}"),
                              ("verify-windows-final", "${{ matrix.artifact }}")):
            with self.subTest(job=job):
                downloads = [step for step in jobs[job]["steps"]
                             if step.get("uses", "").startswith("actions/download-artifact@")]
                self.assertEqual(len(downloads), 1)
                self.assertEqual(downloads[0]["with"].get("name"), expected)
                self.assertNotIn("pattern", downloads[0]["with"])

    def test_every_windows_runtime_crosses_each_signing_boundary(self) -> None:
        names = (
            "Prepare isolated files for signing",
            "Sign runtime inputs with fallback PFX",
            "Verify every signed runtime input without executing it",
            "Upload signed runtime inputs",
            "Restore signed inputs into packaging workspace",
            "Verify signed contents of portable ZIP before MSI signing",
            "Verify, execute, install, and uninstall exact final package",
        )
        for name in names:
            with self.subTest(step=name):
                step = workflow_step("release.yml", name)
                text = step.get("run", step.get("with", {}).get("path", ""))
                for runtime in RUNTIMES:
                    self.assertIn(runtime + ".exe", text)

    def test_every_macos_runtime_is_universal_and_signed(self) -> None:
        for name in (
            "Assemble and smoke-test unsigned universal app",
            "Restore unsigned app without checking out repository source",
            "Sign, notarize, and staple universal DMG",
            "Verify, mount and execute exact notarized DMG",
        ):
            with self.subTest(step=name):
                script = workflow_step("release.yml", name)["run"]
                self.assertIn("for runtime in " + " ".join(RUNTIMES), script)
        nightly = yaml.safe_load((ROOT / ".github/workflows/nightly.yml").read_text(encoding="utf-8"))
        scripts = "\n".join(step.get("run", "") for step in nightly["jobs"]["unsigned-macos"]["steps"])
        self.assertIn("for runtime in " + " ".join(RUNTIMES), scripts)

    def test_helper_is_not_invoked_with_version(self) -> None:
        for workflow in ("release.yml", "nightly.yml"):
            text = (ROOT / ".github/workflows" / workflow).read_text(encoding="utf-8")
            self.assertNotRegex(text, r"(?:suggestion|ssh)-helper(?:\.exe)?[\"']?\s+--version")
        helper = (ROOT / "apps/automexia-terminal/src/bin/automexia-suggestion-helper.rs").read_text(encoding="utf-8")
        self.assertIn("std::env::args_os().len() != 1", helper)

    def test_linux_packaging_restores_every_staged_runtime(self) -> None:
        for missing in (None, *RUNTIMES):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory(prefix="automexia-linux-restore-") as directory:
                root = Path(directory)
                for runtime in RUNTIMES:
                    if runtime != missing:
                        write_fixture_script(root / "staged/fixture-artifact" / runtime, runtime_fixture(runtime))
                write_fixture_script(root / "fixture-bin/cargo", '''[[ "$*" == 'xtask package --target fixture-target' ]]
[[ "$AUTOMEXIA_PACKAGE_SKIP_BUILD" == 1 ]]
printf '%s\\n' "$*" >> "$FIXTURE_TRACE/cargo"
''')
                write_fixture_script(root / "fixture-bin/git", "[[ \"$*\" == 'log -1 --format=%ct' ]]\nprintf '%s\\n' 1700000000\n")
                for tool in ("desktop-file-validate", "appstreamcli", "lintian"):
                    write_fixture_script(root / "fixture-bin" / tool, f"printf '%s\\n' '{tool}' >> \"$FIXTURE_TRACE/validators\"\n")
                result = self.run_fixture_step(root, "Build DEB, RPM, and portable archive")
                if missing is not None:
                    self.assertNotEqual(result.returncode, 0, "incomplete Linux runtime was packaged")
                    self.assertFalse((root / "trace/cargo").exists(), "packaging ran before missing-runtime rejection")
                    continue
                self.assertEqual(result.returncode, 0, result.stderr)
                target = root / "target/fixture-target/release"
                self.assertEqual({path.name for path in target.iterdir()}, set(RUNTIMES))
                for runtime in RUNTIMES:
                    self.assertEqual((target / runtime).read_bytes(), (root / "staged/fixture-artifact" / runtime).read_bytes())
                self.assertEqual((root / "trace/cli").read_text().splitlines(), ["automexia", "amx"])
                self.assertEqual((root / "trace/cargo").read_text().splitlines(), ["xtask package --target fixture-target"])
                self.assertEqual((root / "trace/validators").read_text().splitlines(), ["desktop-file-validate", "appstreamcli", "lintian"])

    def test_macos_assembly_requires_both_slices_of_every_runtime(self) -> None:
        cases = [None, *((arch, runtime) for arch in ("x86_64", "arm64") for runtime in RUNTIMES)]
        for workflow, missing in ((workflow, missing) for workflow in ("release.yml", "first-alpha.yml") for missing in cases):
            with self.subTest(workflow=workflow, missing=missing), tempfile.TemporaryDirectory(prefix="automexia-macos-assemble-") as directory:
                root = Path(directory)
                for arch in ("x86_64", "arm64"):
                    for runtime in RUNTIMES:
                        if (arch, runtime) != missing:
                            write_fixture_script(root / f"staged/{arch}/macos-{arch}" / runtime, runtime_fixture(runtime))
                write_fixture_script(root / "fixture-bin/lipo", '''case "$1" in
  -create)
    [[ "$#" -eq 5 && "$4" == -output ]]
    test -f "$2" && test -f "$3"
    printf '%s\\n' "$2|$3" >> "$FIXTURE_TRACE/lipo-create"
    cp "$2" "$5"
    ;;
  *)
    # Native lipo consumes every token after -verify_arch as an architecture.
    [[ "$#" -eq 4 && "$2" == -verify_arch && "$3" == x86_64 && "$4" == arm64 ]]
    test -x "$1"
    printf '%s\\n' "${1##*/}" >> "$FIXTURE_TRACE/lipo-verify"
    ;;
esac
''')
                write_fixture_script(root / "fixture-bin/file", '''[[ "$#" -eq 1 ]]
test -x "$1"
printf '%s: universal binary\\n' "$1"
''')
                write_fixture_script(root / "fixture-bin/codesign", '''[[ "$#" -eq 4 && "$1" == --force && "$2" == --sign && "$3" == - ]]
test -x "$4"
printf '%s\\n' "${4##*/}" >> "$FIXTURE_TRACE/adhoc"
''')
                (root / "packaging/macos").mkdir(parents=True)
                shutil.copyfile(ROOT / "packaging/macos/Info.plist", root / "packaging/macos/Info.plist")
                (root / "assets/brand").mkdir(parents=True)
                (root / "assets/brand/automexia-terminal.icns").write_bytes(b"fixture icon\n")
                for document in ("LICENSE", "NOTICE.md", "THIRD_PARTY_NOTICES.md", "README.md"):
                    (root / document).write_text("fixture document\n", encoding="utf-8")
                (root / "shell-integration").mkdir()
                (root / "shell-integration/fixture.txt").write_text("fixture shell resource\n", encoding="utf-8")
                (root / "fixture-temp").mkdir()
                result = self.run_fixture_step(root, "Assemble and smoke-test unsigned universal app", workflow)
                archive = root / "fixture-temp/unsigned-macos-app.tar.gz"
                if missing is not None:
                    self.assertNotEqual(result.returncode, 0, "incomplete macOS runtime was assembled")
                    self.assertFalse(archive.exists(), "upload archive exists despite missing architecture slice")
                    continue
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((root / "trace/lipo-create").read_text().splitlines(),
                                 [f"staged/x86_64/macos-x86_64/{runtime}|staged/arm64/macos-arm64/{runtime}" for runtime in RUNTIMES])
                self.assertEqual((root / "trace/lipo-verify").read_text().splitlines(), list(RUNTIMES))
                self.assertEqual((root / "trace/adhoc").read_text().splitlines(), list(RUNTIMES))
                self.assertEqual((root / "trace/cli").read_text().splitlines(), ["automexia", "amx"])
                app = root / "dist/Automexia Terminal.app/Contents"
                metadata = plistlib.loads((app / "Info.plist").read_bytes())
                self.assertEqual(metadata["CFBundleShortVersionString"], "0.4.0")
                self.assertEqual(metadata["CFBundleVersion"], "0.4.0")
                self.assertEqual(metadata["AutomexiaReleaseVersion"], "0.4.0")
                with tarfile.open(archive) as packaged:
                    files = {entry.name for entry in packaged.getmembers() if entry.isfile()}
                prefix = "Automexia Terminal.app/Contents/"
                self.assertTrue({prefix + "MacOS/" + runtime for runtime in RUNTIMES} <= files)
                self.assertIn(prefix + "Resources/shell-integration/fixture.txt", files)
                self.assertIn(prefix + "Resources/README.md", files)

    def test_linux_reproducibility_invokes_non_executable_wrapper_with_bash(self) -> None:
        bash = bash_program()
        if bash is None:
            self.skipTest("Bash unavailable for release reproducibility fixture")
        for workflow in ("release.yml", "first-alpha.yml"):
            document = yaml.safe_load((ROOT / ".github/workflows" / workflow).read_text(encoding="utf-8"))
            job = document["jobs"]["reproducibility-linux"]
            script = next(step["run"] for step in job["steps"]
                          if "check_reproducible_build.sh" in step.get("run", ""))
            for arch, host in (("x86_64", "x86_64-unknown-linux-gnu"),
                               ("aarch64", "aarch64-unknown-linux-gnu")):
                with self.subTest(workflow=workflow, arch=arch), tempfile.TemporaryDirectory(prefix="automexia-repro-permission-") as directory:
                    root = Path(directory)
                    wrapper = root / "tools/ci/check_reproducible_build.sh"
                    wrapper.parent.mkdir(parents=True)
                    wrapper.write_text((ROOT / "tools/ci/check_reproducible_build.sh").read_text(encoding="utf-8"), encoding="utf-8", newline="\n")
                    wrapper.chmod(0o644)  # Match its committed mode, never make it executable.
                    write_fixture_script(root / "fixture-bin/git", '[[ "$*" == "rev-parse --show-toplevel" ]]\npwd\n')
                    write_fixture_script(root / "fixture-bin/rustc", '[[ "$1" == -vV ]]\nprintf "host: %s\\n" "$FIXTURE_HOST"\n')
                    write_fixture_script(root / "fixture-bin/python3", '[[ "$#" -eq 6 ]]\nprintf "%s\\n" "$@" > trace-argv\n')
                    command = script.replace("${{ matrix.arch }}", arch).replace("${{ matrix.artifact }}", "fixture artifact")
                    result = subprocess.run([bash, "-c", 'export PATH="$PWD/fixture-bin:$PATH"\n' + command],
                                            cwd=root, env={**os.environ, "FIXTURE_HOST": host},
                                            capture_output=True, text=True, timeout=15, check=False)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    args = (root / "trace-argv").read_text(encoding="utf-8").splitlines()
                    self.assertTrue(args[0].endswith("/tools/ci/reproducible_build.py"))
                    self.assertEqual(args[1:], [f"trust/evidence/reproducibility-{arch}.json", "--target", host,
                                               "--reference", "reference/fixture artifact"])
                    # DrvFS/Windows may emulate executable bits; retain the invocation contract there.
                    self.assertRegex(script, r"(?m)^\s*bash tools/ci/check_reproducible_build\.sh\s")

    def test_final_linux_verifier_installs_a_local_file_not_an_apt_package_name(self) -> None:
        for workflow in ("release.yml", "first-alpha.yml"):
            with self.subTest(workflow=workflow), tempfile.TemporaryDirectory(prefix="automexia-apt-path-") as directory:
                root = Path(directory)
                artifacts = root / "artifacts/packages with spaces"
                artifacts.mkdir(parents=True)
                for suffix in ("deb", "rpm"):
                    (artifacts / ("automexia-x86_64." + suffix)).write_bytes(b"fixture package")
                payload = root / "payload/automexia"
                write_fixture_script(payload, runtime_fixture("automexia"))
                with tarfile.open(artifacts / "automexia-x86_64.tar.gz", "w:gz") as archive:
                    archive.add(payload, arcname="automexia")
                for tool in ("file", "readelf", "dpkg-deb", "rpm"):
                    write_fixture_script(root / "fixture-bin" / tool, ":\n")
                write_fixture_script(root / "fixture-bin/automexia", runtime_fixture("automexia"))
                write_fixture_script(root / "fixture-bin/sudo", '''[[ "$#" -eq 4 && "$1" == apt-get && "$2" == install && "$3" == -y ]]
[[ "$4" == /* && "$4" == *.deb && -f "$4" ]]
printf '%s\n' "$4" >> "$FIXTURE_TRACE/install"
''')
                from unittest import mock
                with mock.patch.dict(os.environ, {"TARGET_REGEX": "x86_64"}):
                    result = self.run_fixture_step(root, "Verify tar, DEB and RPM assets", workflow)
                self.assertEqual(result.returncode, 0, result.stderr)
                installed = (root / "trace/install").read_text().splitlines()
                self.assertEqual(len(installed), 1)
                self.assertTrue(installed[0].endswith("/artifacts/packages with spaces/automexia-x86_64.deb"))

    def test_windows_signing_staging_excludes_vendor_binaries(self) -> None:
        pwsh = shutil.which("pwsh")
        if pwsh is None or shutil.which("tar") is None:
            self.skipTest("PowerShell/tar unavailable for signing-input fixture")
        with tempfile.TemporaryDirectory(prefix="automexia-signing-input-") as directory:
            root = Path(directory)
            source = root / "staged-binary/runtime"
            source.mkdir(parents=True)
            product = {runtime + ".exe" for runtime in RUNTIMES}
            for relative in (*product, "conpty.dll", "x64/OpenConsole.exe", "arm64/OpenConsole.exe"):
                path = source / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fictional unsigned fixture; never execute")
            scripts = root / "source/shell-integration"
            scripts.mkdir(parents=True)
            (scripts / "fixture.ps1").write_text("# fixture; never execute\n", encoding="utf-8")
            (root / "staged-source").mkdir()
            with tarfile.open(root / "staged-source/windows-package-inputs.tar.gz", "w:gz") as archive:
                archive.add(scripts, arcname="shell-integration")
            script = root / "exercise.ps1"
            script.write_text(workflow_step("release.yml", "Prepare isolated files for signing")["run"],
                              encoding="utf-8")
            result = subprocess.run([pwsh, "-NoProfile", "-File", str(script)], cwd=root,
                                    capture_output=True, text=True, timeout=30, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            files = {path.relative_to(root / "signing-input").as_posix()
                     for path in (root / "signing-input").rglob("*") if path.is_file()}
            self.assertEqual(files, product | {"shell-integration/fixture.ps1"})

    def test_vendor_verification_precedes_final_windows_execution(self) -> None:
        workflow = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8"))
        job = workflow["jobs"]["verify-windows-final"]
        checkout = next(step for step in job["steps"] if step.get("uses", "").startswith("actions/checkout@"))
        self.assertEqual(checkout["with"]["ref"], "${{ needs.preflight.outputs.commit }}")
        self.assertFalse(checkout["with"]["persist-credentials"])
        source = workflow_step("release.yml", "Verify, execute, install, and uninstall exact final package")["run"]
        for root in ("$portable", "$installed.DirectoryName"):
            with self.subTest(root=root):
                self.assertLess(source.index(f"Assert-ConPtyRuntime -Root {root}"),
                                source.index(f"& (Join-Path {root} $runtime) --version"))
        sbom = workflow_step("release.yml", "Bind pinned Microsoft transport to both SBOM formats")
        for required in (".github/scripts/build_release_manifest.py", "--prepare-sbom",
                         "--source sbom-raw", "--output release-assets", "--scan-root sbom-input"):
            self.assertIn(required, sbom["run"])
        # The manifest CLI's native fixture verifies the resulting vendor records,
        # application identity and privacy projection in both public SBOM formats.

    def test_vendor_verifier_checks_hashes_before_each_native_signature(self) -> None:
        pwsh = shutil.which("pwsh")
        if pwsh is None:
            self.skipTest("PowerShell unavailable for vendor signature fixture")
        harness = r'''
param([string]$Verifier, [string]$FixtureRoot, [string]$Architecture, [string]$Mode, [string]$Expected)
$ErrorActionPreference = 'Stop'
. $Verifier
$script:hashChecks = 0
$script:signatureChecks = 0
function python {
    if ($args.Count -ne 6 -or $args[1] -cne 'verify' -or
        $args[2] -cne '--destination' -or $args[3] -cne $FixtureRoot -or
        $args[4] -cne '--architecture' -or $args[5] -cne $Architecture) {
        throw 'vendor hash/PE verifier arguments changed'
    }
    $script:hashChecks++
    $global:LASTEXITCODE = if ($Mode -eq 'hash-failure') { 1 } else { 0 }
}
function Get-AuthenticodeSignature {
    param([string]$LiteralPath)
    if ($script:hashChecks -ne 1) { throw 'native signature inspected before bounded hash verification' }
    $script:signatureChecks++
    $usages = [Security.Cryptography.OidCollection]::new()
    if ($Mode -ne 'no-eku') { $usages.Add([Security.Cryptography.Oid]::new('1.3.6.1.5.5.7.3.3')) | Out-Null }
    $certificate = [pscustomobject]@{
        # Match the native PowerShell adapter: ObjectId is a string, not an Oid.
        EnhancedKeyUsageList = @([pscustomobject]@{ ObjectId = '1.3.6.1.5.5.7.3.3' })
        Extensions = @([Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new($usages, $false))
    }
    $certificate | Add-Member ScriptMethod GetNameInfo {
        param($Type, $Issuer)
        if ($Mode -eq 'product-publisher') { return 'Automexia Test' }
        return 'Microsoft Corporation'
    }
    $status = if ($Mode -eq 'invalid-signature' -or
                  ($Mode -eq 'invalid-third-file' -and $script:signatureChecks -eq 3)) {
        'HashMismatch'
    } else { 'Valid' }
    [pscustomobject]@{
        Status = $status
        SignerCertificate = $(if ($Mode -eq 'no-certificate') { $null } else { $certificate })
        TimeStamperCertificate = $(if ($Mode -eq 'no-timestamp') { $null } else { @{ Subject = 'Vendor timestamp' } })
    }
}
try {
    $files = @(Assert-ConPtyRuntime -Root $FixtureRoot -Architecture $Architecture)
    if ($Expected -ne 'pass') { throw 'vendor verifier accepted invalid fixture' }
    if (($files -join '|') -cne 'conpty.dll|x64/OpenConsole.exe|arm64/OpenConsole.exe') {
        throw 'vendor verifier omitted or reordered required runtime assets'
    }
    if ($script:signatureChecks -ne 3) { throw 'vendor verifier skipped a native signature' }
} catch {
    if ($Expected -eq 'pass' -or $_.Exception.Message -notmatch $Expected) { throw }
}
if ($script:hashChecks -ne 1) { throw 'vendor verifier skipped exact file hash validation' }
if ($Mode -eq 'hash-failure' -and $script:signatureChecks -ne 0) {
    throw 'hash failure proceeded into signature verification'
}
'''
        cases = [("x64", "valid", "pass"), ("arm64", "valid", "pass")]
        cases += [("x64", mode, message) for mode, message in (
            ("hash-failure", "hash/architecture"),
            ("invalid-signature", "signature is invalid"),
            ("invalid-third-file", "signature is invalid"),
            ("no-certificate", "signature is invalid"),
            ("product-publisher", "publisher mismatch"),
            ("no-timestamp", "timestamp is missing"),
            ("no-eku", "code-signing EKU is missing"),
        )]
        with tempfile.TemporaryDirectory(prefix="automexia-vendor-trust-") as directory:
            root = Path(directory)
            script = root / "exercise.ps1"
            script.write_text(harness, encoding="utf-8")
            for architecture, mode, expected in cases:
                with self.subTest(architecture=architecture, mode=mode):
                    result = subprocess.run(
                        [pwsh, "-NoProfile", "-File", str(script), "-Verifier",
                         str(ROOT / "tools/ci/windows_conpty_trust.ps1"), "-FixtureRoot", str(root),
                         "-Architecture", architecture, "-Mode", mode, "-Expected", expected],
                        capture_output=True, text=True, timeout=30, check=False,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)

    def test_product_signature_uses_typed_certificate_eku(self) -> None:
        pwsh = shutil.which("pwsh")
        if pwsh is None:
            self.skipTest("PowerShell unavailable for product certificate fixture")
        source = (ROOT / "tools/ci/test_release_trust_windows.ps1").read_text(encoding="utf-8")
        function = "function Assert-TrustedSignature {" + source.split(
            "function Assert-TrustedSignature {", 1
        )[1].split("function Expand-TrustedPortableArchive", 1)[0]
        harness = r'''
param([string]$Mode)
$ErrorActionPreference = 'Stop'
$ExpectedPublisher = 'CN=Automexia Test'
$signatures = [System.Collections.Generic.List[object]]::new()
function Get-AuthenticodeSignature {
    param([string]$LiteralPath)
    $usages = [Security.Cryptography.OidCollection]::new()
    if ($Mode -eq 'valid') { $usages.Add([Security.Cryptography.Oid]::new('1.3.6.1.5.5.7.3.3')) | Out-Null }
    [pscustomobject]@{
        Status = 'Valid'
        SignerCertificate = [pscustomobject]@{
            Subject = 'CN=Automexia Test'; Thumbprint = 'fixture'
            EnhancedKeyUsageList = @([pscustomobject]@{ ObjectId = '1.3.6.1.5.5.7.3.3' })
            Extensions = @([Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new($usages, $false))
        }
        TimeStamperCertificate = @{ Subject = 'Fixture timestamp' }
    }
}
'''
        harness += function
        harness += r'''
try {
    Assert-TrustedSignature -Path 'fixture.exe'
    if ($Mode -ne 'valid') { throw 'product signature accepted missing certificate EKU' }
    if ($signatures.Count -ne 1) { throw 'product signature evidence was not recorded' }
} catch {
    if ($Mode -eq 'valid' -or $_.Exception.Message -notmatch 'code-signing extended key usage') { throw }
    if ($signatures.Count -ne 0) { throw 'invalid product signature produced evidence' }
}
'''
        with tempfile.TemporaryDirectory(prefix="automexia-product-trust-") as directory:
            script = Path(directory) / "exercise.ps1"
            script.write_text(harness, encoding="utf-8")
            for mode in ("valid", "no-eku"):
                with self.subTest(mode=mode):
                    result = subprocess.run([pwsh, "-NoProfile", "-File", str(script), "-Mode", mode],
                                            capture_output=True, text=True, timeout=30, check=False)
                    self.assertEqual(result.returncode, 0, result.stderr)

    def test_archive_reader_rejects_missing_extra_duplicate_and_unsafe_entries(self) -> None:
        pwsh = shutil.which("pwsh") or shutil.which("powershell")
        if pwsh is None:
            self.skipTest("PowerShell unavailable for native archive-reader regression")
        source = (ROOT / "tools/ci/test_release_trust_windows.ps1").read_text(encoding="utf-8")
        # Load only the repository-owned archive reader. Never execute its top
        # level signing/Defender workflow or obtain production certificates.
        function = source.split("function Expand-TrustedPortableArchive {", 1)[1].split("function Find-DefenderScanner", 1)[0]
        function = "function Expand-TrustedPortableArchive {" + function
        harness = r'''
param([string]$Archive, [string]$Expected)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$MaximumArchiveEntries = 4096
$Version = '0.4.0'
$temporaryRoots = [System.Collections.Generic.List[string]]::new()
$script:vendorSignatureCount = 0
$scanRoot = [IO.Path]::GetDirectoryName($Archive)
$script:checkedScripts = [System.Collections.Generic.List[string]]::new()
# Fixture bytes have no PE metadata/signatures. Stub these native boundaries,
# retaining real archive parsing, allowlist validation, and bounded extraction.
function Get-Item {
    param([string]$LiteralPath)
    $item = Microsoft.PowerShell.Management\Get-Item -LiteralPath $LiteralPath
    [pscustomobject]@{ FullName = $item.FullName; VersionInfo = @{ ProductVersion = '0.4.0' } }
}
function Assert-ConPtyRuntime {
    param([string]$Root, [string]$Architecture)
    return @('conpty.dll', 'x64/OpenConsole.exe', 'arm64/OpenConsole.exe')
}
function Assert-PowerShellEditorRuntime {
    param([string]$Root)
    $recipe = Get-Content -LiteralPath (Join-Path $PSScriptRoot '../../packaging/windows/powershell-editor.json') -Raw | ConvertFrom-Json
    return @($recipe.files | ForEach-Object { 'powershell-modules/PSReadLine/' + $recipe.package.version + '/' + $_.path })
}
function Assert-TrustedSignature {
    param([string]$Path, [bool]$RecordEvidence)
    $script:checkedScripts.Add([IO.Path]::GetFileName($Path))
}
'''
        harness += function
        harness += r'''
try {
    try {
        $paths = @(Expand-TrustedPortableArchive -Path $Archive)
        if ($Expected -ne 'pass') { throw 'fixture accepted an invalid archive' }
        [string[]]$names = @($paths | ForEach-Object { [IO.Path]::GetFileName($_) })
        # Windows PowerShell and PowerShell 7 use different cultural punctuation
        # ordering. File identity is an ordinal contract on both platforms.
        [Array]::Sort($names, [StringComparer]::Ordinal)
        if (($names -join '|') -cne 'amx.exe|automexia-ssh-helper.exe|automexia-suggestion-helper.exe|automexia.exe') {
            throw 'archive reader failed to return all four runtime executables'
        }
        if ($script:checkedScripts.Count -ne 8) { throw 'archive reader skipped signed shell assets' }
        if ($script:vendorSignatureCount -ne 10) { throw 'archive reader skipped vendor signatures' }
    } catch {
        if ($Expected -eq 'pass' -or $_.Exception.Message -notmatch $Expected) { throw }
        if ($temporaryRoots.Count -ne 0) { throw 'invalid archive was extracted before rejection' }
    }
} finally {
    # These roots came only from the function exercised above; verify their
    # resolved temporary prefix before removing the fixture's extracted files.
    $prefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar + 'automexia-release-trust-'
    foreach ($root in $temporaryRoots) {
        $full = [IO.Path]::GetFullPath($root)
        if (-not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'unexpected fixture cleanup path' }
        Remove-Item -LiteralPath $full -Recurse -Force
    }
}
'''
        expected = [name + ".exe" for name in RUNTIMES]
        expected += ["LICENSE", "NOTICE.md", "README.md", "THIRD_PARTY_NOTICES.md"]
        vendor = ["conpty.dll", "x64/OpenConsole.exe", "arm64/OpenConsole.exe"]
        recipe = json.loads((ROOT / "packaging/windows/powershell-editor.json").read_text(encoding="utf-8"))
        vendor += ["powershell-modules/PSReadLine/" + recipe["package"]["version"] + "/" + item["path"]
                   for item in recipe["files"]]
        expected += vendor
        expected += ["shell-integration/" + name for name in SHELL_ASSETS]
        cases = [("complete", expected, "pass")]
        cases.extend(("missing-vendor-" + str(index), [name for name in expected if name != asset], "content mismatch")
                     for index, asset in enumerate(vendor))
        cases.extend(("missing-" + runtime, [name for name in expected if name != runtime + ".exe"], "content mismatch") for runtime in RUNTIMES)
        cases.extend((
            ("legacy-main-only", [name for name in expected if name not in ("amx.exe", "automexia-suggestion-helper.exe", "automexia-ssh-helper.exe")], "content mismatch"),
            ("extra-runtime", expected + ["unexpected.exe"], "content mismatch"),
            ("traversal", expected + ["../escape.exe"], "unsafe path"),
            ("duplicate", expected + ["./amx.exe"], "content mismatch"),
        ))
        with tempfile.TemporaryDirectory(prefix="automexia-archive-test-") as directory:
            root = Path(directory)
            script = root / "tools/ci/exercise.ps1"
            script.parent.mkdir(parents=True)
            recipe_path = root / "packaging/windows/powershell-editor.json"
            recipe_path.parent.mkdir(parents=True)
            shutil.copyfile(ROOT / "packaging/windows/powershell-editor.json", recipe_path)
            script.write_text(harness, encoding="utf-8")
            for name, entries, outcome in cases:
                with self.subTest(case=name):
                    archive = root / (name + ".zip")
                    with zipfile.ZipFile(archive, "w") as output:
                        for entry in entries:
                            output.writestr(entry, "fixture\n")
                    result = subprocess.run([pwsh, "-NoProfile", "-ExecutionPolicy", "RemoteSigned", "-File", str(script), "-Archive", str(archive), "-Expected", outcome],
                                            capture_output=True, text=True, timeout=30, check=False)
                    self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
