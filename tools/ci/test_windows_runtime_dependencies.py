"""Reject Windows packages that depend on an undeployed Visual C++ runtime."""

import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import qa_process
import windows_runtime_dependencies as dependencies


ROOT = Path(__file__).resolve().parents[2]


def dumpbin_output(*imports):
    return ("File Type: EXECUTABLE IMAGE\n\n"
            "  Image has the following dependencies:\n\n"
            + "".join(f"    {name}\n" for name in imports)
            + "\n  Summary\n\n    1000 .text\n")


class WindowsRuntimeDependenciesTests(unittest.TestCase):
    def test_detects_reported_imports_and_runtime_variants(self):
        for name in ("MSVCP140.dll", "VCRUNTIME140.dll", "VCRUNTIME140_1.dll",
                     "msvcp140_atomic_wait.dll", "MSVCP140D.dll", "MSVCR120.dll",
                     "CONCRT140.dll", "VCOMP140.dll", "VCCORLIB140.dll", "ucrtbased.dll"):
            with self.subTest(name=name):
                imports = dependencies.parse_dependencies(dumpbin_output("KERNEL32.dll", name))
                self.assertEqual(dependencies.external_crt(imports), [name.lower()])

    def test_accepts_windows_system_runtime_and_delay_imports_are_checked(self):
        output = dumpbin_output("KERNEL32.dll", "ucrtbase.dll", "msvcp_win.dll",
                                "api-ms-win-crt-runtime-l1-1-0.dll")
        self.assertEqual(dependencies.external_crt(dependencies.parse_dependencies(output)), [])
        output = output.replace("  Summary", "  Image has the following delay load dependencies:\n"
                                "\n    VCRUNTIME140_1.dll\n\n  Summary")
        self.assertEqual(dependencies.external_crt(dependencies.parse_dependencies(output)),
                         ["vcruntime140_1.dll"])

    def test_rejects_empty_invalid_or_incomplete_inspection(self):
        for output in ("", "File Type: LIBRARY\nKERNEL32.dll", "KERNEL32.dll",
                       dumpbin_output(), dumpbin_output("KERNEL32.dll").replace("Summary", "")):
            with self.subTest(output=output):
                with self.assertRaisesRegex(RuntimeError, "unrecognized"):
                    dependencies.parse_dependencies(output)

    def test_capture_bounds_output_and_fails_on_process_errors(self):
        for result in (qa_process.Result(1, False, None), qa_process.Result(None, True, None),
                       qa_process.Result(0, False, "cleanup incomplete")):
            with mock.patch.object(dependencies.qa_process, "run", return_value=result):
                with self.assertRaisesRegex(RuntimeError, "inspection tool failed"):
                    dependencies.capture(["dumpbin.exe"])

        def flood(command, **kwargs):
            kwargs["consume"](b"x" * (dependencies.MAX_OUTPUT_BYTES + 1))
            return qa_process.Result(0, False, None)

        with mock.patch.object(dependencies.qa_process, "run", side_effect=flood):
            with self.assertRaisesRegex(RuntimeError, "output limit"):
                dependencies.capture(["dumpbin.exe"])

    def test_capture_uses_bounded_owner_and_english_tool_output(self):
        def run(command, **kwargs):
            self.assertEqual(command, ["dumpbin.exe", "/dependents", "file with spaces.exe"])
            self.assertEqual(kwargs["environment"]["VSLANG"], "1033")
            self.assertEqual(kwargs["timeout_seconds"], 30)
            kwargs["consume"](b"first")
            kwargs["consume"](b" second")
            return qa_process.Result(0, False, None)

        with mock.patch.object(dependencies.qa_process, "run", side_effect=run):
            self.assertEqual(dependencies.capture(["dumpbin.exe", "/dependents", "file with spaces.exe"]),
                             "first second")

    def test_every_supplied_binary_is_checked_and_failures_block_packaging(self):
        with tempfile.TemporaryDirectory() as directory:
            binaries = [Path(directory) / name for name in ("automexia.exe", "amx.exe", "conpty.dll")]
            for binary in binaries:
                binary.write_bytes(b"fixture")
            good = dumpbin_output("KERNEL32.dll")
            with mock.patch.object(dependencies, "capture", return_value=good) as capture:
                dependencies.check_binaries(binaries, Path("dumpbin.exe"))
                self.assertEqual(capture.call_count, 3)
                self.assertEqual(capture.call_args_list[-1].args[0][-1], str(binaries[-1].resolve()))
            with mock.patch.object(dependencies, "capture", side_effect=[good, dumpbin_output("VCRUNTIME140.dll")]):
                with self.assertRaisesRegex(RuntimeError, "amx.exe.*vcruntime140.dll"):
                    dependencies.check_binaries(binaries, Path("dumpbin.exe"))
            binaries[-1].unlink()
            with mock.patch.object(dependencies, "capture") as capture:
                with self.assertRaisesRegex(RuntimeError, "missing or oversized"):
                    dependencies.check_binaries(binaries, Path("dumpbin.exe"))
                capture.assert_not_called()

    def test_empty_or_excessive_inventory_and_missing_inspector_fail(self):
        for binaries in ([], [Path("app.exe")] * 17):
            with self.assertRaisesRegex(RuntimeError, "binary inventory"):
                dependencies.check_binaries(binaries, Path("dumpbin.exe"))
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(RuntimeError, "DUMPBIN"):
                dependencies.find_dumpbin(Path(directory) / "missing.exe")

    def test_inspector_discovery_uses_host_architecture_and_exact_arguments(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            locator = base / "Microsoft Visual Studio/Installer/vswhere.exe"
            locator.parent.mkdir(parents=True)
            locator.touch()
            inspector = base / "dumpbin.exe"
            inspector.touch()
            for architecture, expected in (("AMD64", "x64"), ("ARM64", "arm64")):
                with mock.patch.object(dependencies.shutil, "which", return_value=None), \
                        mock.patch.dict(dependencies.os.environ, {"ProgramFiles(x86)": directory}), \
                        mock.patch.object(dependencies.platform, "machine", return_value=architecture), \
                        mock.patch.object(dependencies, "capture", return_value=str(inspector) + "\n") as capture:
                    self.assertEqual(dependencies.find_dumpbin(), inspector.resolve())
                    self.assertEqual(capture.call_args.args[0], [str(locator), "-latest", "-products", "*", "-find",
                                     rf"VC\Tools\MSVC\**\bin\Host{expected}\{expected}\dumpbin.exe", "-utf8"])

    def test_packaging_checks_both_formats_after_vendor_staging_even_without_build(self):
        source = (ROOT / "tools/xtask/src/main.rs").read_text(encoding="utf-8")
        body = source.split("fn package_target(target:", 1)[1].split("fn package_native_desktop(", 1)[0]
        self.assertLess(body.index("prepare_windows_runtime(runtime_root, target)?;"),
                        body.index("check_windows_runtime_dependencies(&binary, target)?;"))
        self.assertLess(body.index("check_windows_runtime_dependencies(&binary, target)?;"),
                        body.index("package_windows_arm64("))
        self.assertLess(body.index("check_windows_runtime_dependencies(&binary, target)?;"),
                        body.index("package_native_desktop("))
        self.assertLess(body.index("check_windows_runtime_dependencies(&binary, target)?;"),
                        body.index("portable_archive("))
        self.assertNotIn("skip_build", body[body.index('if target.contains("windows")'):])
        owner = source.split("fn check_windows_runtime_dependencies(", 1)[1].split("\nfn ", 1)[0]
        self.assertIn("package_runtime_binaries(binary, target)", owner)
        self.assertIn("package_conpty_runtime_files(binary)?", owner)
        for workflow in ("ci.yml", "first-alpha.yml", "release.yml", "nightly.yml"):
            self.assertIn("-p test_windows_runtime_dependencies.py",
                          (ROOT / ".github/workflows" / workflow).read_text(encoding="utf-8"))

    @unittest.skipUnless(os.name == "nt", "requires a disposable native Windows builder")
    def test_native_windows_dynamic_crt_fails_and_static_crt_passes(self):
        # Compile actual imports; never execute either candidate executable.
        inspector = dependencies.find_dumpbin()
        host = next(line.removeprefix("host: ") for line in dependencies.capture(["rustc", "-vV"]).splitlines()
                    if line.startswith("host: "))
        self.assertIn("windows-msvc", host)
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "runtime.rs"
            source.write_text('fn main() { println!("runtime fixture"); }\n', encoding="utf-8")
            for mode in ("-", "+"):
                binary = Path(directory) / ("dynamic.exe" if mode == "-" else "static.exe")
                dependencies.capture(["rustc", str(source), "--target", host,
                                      f"-Ctarget-feature={mode}crt-static", "-o", str(binary)], timeout=120)
                if mode == "-":
                    with self.assertRaisesRegex(RuntimeError, "external Visual C"):
                        dependencies.check_binaries([binary], inspector)
                else:
                    dependencies.check_binaries([binary], inspector)


if __name__ == "__main__":
    unittest.main()
