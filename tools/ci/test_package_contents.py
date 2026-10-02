"""Keep runtime companions reachable in each declarative package format."""

from pathlib import Path
import tomllib
import unittest
import xml.etree.ElementTree as ET

import yaml


ROOT = Path(__file__).resolve().parents[2]
RUNTIME_BINARIES = ("automexia", "amx", "automexia-suggestion-helper", "automexia-ssh-helper")


class PackageContentsTests(unittest.TestCase):
    def test_windows_installers_include_the_exact_vendor_layout(self):
        for name, namespace, source in (
            ("automexia.wxs", "http://schemas.microsoft.com/wix/2006/wi", "$(env.AUTOMEXIA_CONPTY_ROOT)"),
            ("automexia-arm64.wxs", "http://wixtoolset.org/schemas/v4/wxs", "$(var.ConptyRuntimeRoot)"),
        ):
            with self.subTest(installer=name):
                document = ET.parse(ROOT / "packaging/windows" / name)
                ns = {"w": namespace}
                components = document.findall(".//w:Component", ns)
                references = [item.attrib["Id"] for item in document.findall(".//w:ComponentRef", ns)]
                for relative in ("conpty.dll", "x64\\OpenConsole.exe", "arm64\\OpenConsole.exe"):
                    matches = [(component, file) for component in components
                               for file in component.findall("w:File", ns)
                               if file.attrib.get("Source") == source + "\\" + relative]
                    self.assertEqual(len(matches), 1, relative)
                    self.assertEqual(references.count(matches[0][0].attrib["Id"]), 1)
                root = document.find(".//w:DirectoryRef[@Id='INSTALLDIR']", ns)
                if root is None:
                    root = document.find(".//w:Directory[@Id='INSTALLFOLDER']", ns)
                self.assertIsNotNone(root)
                for arch in ("x64", "arm64"):
                    self.assertIsNotNone(root.find(f"w:Directory[@Name='{arch}']/w:Component/w:File[@Name='OpenConsole.exe']", ns))
                self.assertIsNone(root.find("w:Component/w:File[@Name='OpenConsole.exe']", ns))

    def test_cargo_packager_declares_each_runtime_binary_once(self):
        manifest = tomllib.loads(
            (ROOT / "apps/automexia-terminal/Cargo.toml").read_text(encoding="utf-8")
        )
        binaries = manifest["package"]["metadata"]["packager"]["binaries"]
        self.assertCountEqual([entry["path"] for entry in binaries], RUNTIME_BINARIES)
        self.assertEqual(
            [entry["path"] for entry in binaries if entry.get("main")], ["automexia"]
        )

    def test_linux_installs_each_runtime_binary_as_executable(self):
        package = yaml.safe_load(
            (ROOT / "packaging/linux/nfpm.yaml").read_text(encoding="utf-8")
        )
        sources = (
            "${AUTOMEXIA_BINARY}",
            "${AUTOMEXIA_CLI_BINARY}",
            "${AUTOMEXIA_SUGGESTION_HELPER_BINARY}",
            "${AUTOMEXIA_SSH_HELPER_BINARY}",
        )
        self.assertEqual(len(sources), len(RUNTIME_BINARIES))
        for name, source in zip(RUNTIME_BINARIES, sources):
            with self.subTest(binary=name):
                matches = [
                    entry for entry in package["contents"]
                    if entry.get("dst") == f"/usr/bin/{name}"
                ]
                self.assertEqual(len(matches), 1)
                self.assertEqual(matches[0]["src"], source)
                self.assertIs(matches[0]["expand"], True)
                self.assertEqual(matches[0]["file_info"]["mode"], 0o755)

    def test_arm64_msi_references_each_companion_component(self):
        document = ET.parse(ROOT / "packaging/windows/automexia-arm64.wxs")
        ns = {"w": "http://wixtoolset.org/schemas/v4/wxs"}
        references = [entry.attrib["Id"] for entry in document.findall(".//w:Feature/w:ComponentRef", ns)]
        sources = ("BinaryPath", "CliBinaryPath", "SuggestionHelperPath", "SshHelperPath")
        self.assertEqual(len(sources), len(RUNTIME_BINARIES))
        for name, source in zip(RUNTIME_BINARIES, sources):
            with self.subTest(binary=name):
                components = [
                    entry for entry in document.findall(".//w:Component", ns)
                    if any(file.attrib.get("Name") == f"{name}.exe" for file in entry.findall("w:File", ns))
                ]
                self.assertEqual(len(components), 1)
                component = components[0]
                self.assertEqual(component.attrib["Bitness"], "always64")
                self.assertEqual(references.count(component.attrib["Id"]), 1)
                file = component.find("w:File", ns)
                self.assertEqual(file.attrib["Source"], f"$(var.{source})")
                self.assertEqual(file.attrib["KeyPath"], "yes")


if __name__ == "__main__":
    unittest.main()
