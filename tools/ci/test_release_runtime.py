#!/usr/bin/env python3
"""Exercise release runtime staging without building, signing, or installing."""

from __future__ import annotations

import os
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
    "fish/automexia.fish", "install-unix.sh", "install-windows.ps1", "posix/automexia-eza-filter.pl",
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
    def run_fixture_step(self, root: Path, name: str) -> subprocess.CompletedProcess:
        bash = bash_program()
        if bash is None:
            self.skipTest("Bash unavailable for workflow package fixture")
        script = workflow_step("release.yml", name)["run"]
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
        for missing in cases:
            with self.subTest(missing=missing), tempfile.TemporaryDirectory(prefix="automexia-macos-assemble-") as directory:
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
  -verify_arch)
    [[ "$#" -eq 4 && "$2" == x86_64 && "$3" == arm64 ]]
    test -x "$4"
    printf '%s\\n' "${4##*/}" >> "$FIXTURE_TRACE/lipo-verify"
    ;;
  *) exit 92 ;;
esac
''')
                write_fixture_script(root / "fixture-bin/file", '''[[ "$#" -eq 1 ]]
test -x "$1"
printf '%s: universal binary\\n' "$1"
''')
                (root / "packaging/macos").mkdir(parents=True)
                (root / "packaging/macos/Info.plist").write_text("version=@AUTOMEXIA_VERSION@\n", encoding="utf-8")
                (root / "assets/brand").mkdir(parents=True)
                (root / "assets/brand/automexia-terminal.icns").write_bytes(b"fixture icon\n")
                for document in ("LICENSE", "NOTICE.md", "THIRD_PARTY_NOTICES.md", "README.md"):
                    (root / document).write_text("fixture document\n", encoding="utf-8")
                (root / "shell-integration").mkdir()
                (root / "shell-integration/fixture.txt").write_text("fixture shell resource\n", encoding="utf-8")
                (root / "fixture-temp").mkdir()
                result = self.run_fixture_step(root, "Assemble and smoke-test unsigned universal app")
                archive = root / "fixture-temp/unsigned-macos-app.tar.gz"
                if missing is not None:
                    self.assertNotEqual(result.returncode, 0, "incomplete macOS runtime was assembled")
                    self.assertFalse(archive.exists(), "upload archive exists despite missing architecture slice")
                    continue
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((root / "trace/lipo-create").read_text().splitlines(),
                                 [f"staged/x86_64/macos-x86_64/{runtime}|staged/arm64/macos-arm64/{runtime}" for runtime in RUNTIMES])
                self.assertEqual((root / "trace/lipo-verify").read_text().splitlines(), list(RUNTIMES))
                self.assertEqual((root / "trace/cli").read_text().splitlines(), ["automexia", "amx"])
                app = root / "dist/Automexia Terminal.app/Contents"
                self.assertEqual((app / "Info.plist").read_text(), "version=0.4.0\n")
                with tarfile.open(archive) as packaged:
                    files = {entry.name for entry in packaged.getmembers() if entry.isfile()}
                prefix = "Automexia Terminal.app/Contents/"
                self.assertTrue({prefix + "MacOS/" + runtime for runtime in RUNTIMES} <= files)
                self.assertIn(prefix + "Resources/shell-integration/fixture.txt", files)
                self.assertIn(prefix + "Resources/README.md", files)

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
        self.assertEqual(sbom["run"].strip(),
                         "python3 tools/ci/release_trust.py --add-vendor-sbom release-assets")

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
        pwsh = shutil.which("pwsh")
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
        $names = @($paths | ForEach-Object { [IO.Path]::GetFileName($_) } | Sort-Object)
        if (($names -join '|') -cne 'amx.exe|automexia-ssh-helper.exe|automexia-suggestion-helper.exe|automexia.exe') {
            throw 'archive reader failed to return all four runtime executables'
        }
        if ($script:checkedScripts.Count -ne 8) { throw 'archive reader skipped signed shell assets' }
        if ($script:vendorSignatureCount -ne 3) { throw 'archive reader skipped vendor signatures' }
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
            script = root / "exercise.ps1"
            script.write_text(harness, encoding="utf-8")
            for name, entries, outcome in cases:
                with self.subTest(case=name):
                    archive = root / (name + ".zip")
                    with zipfile.ZipFile(archive, "w") as output:
                        for entry in entries:
                            output.writestr(entry, "fixture\n")
                    result = subprocess.run([pwsh, "-NoProfile", "-File", str(script), "-Archive", str(archive), "-Expected", outcome],
                                            capture_output=True, text=True, timeout=30, check=False)
                    self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
