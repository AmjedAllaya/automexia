"""Exercise the VSIX assembler with inert desktop and companion fixtures."""
import io
import os
from pathlib import Path
import stat
import struct
import tempfile
import unittest
from unittest import mock
import xml.etree.ElementTree as ET
import zipfile

import package_vscode as package


def pe(machine):
    data = bytearray(70)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 60, 64)
    struct.pack_into("<4sH", data, 64, b"PE\0\0", machine)
    return bytes(data)


def elf(machine):
    data = bytearray(64)
    data[:7] = b"\x7fELF\x02\x01\x01"
    struct.pack_into("<HH", data, 16, 3, machine)
    return bytes(data)


def mach(cpu):
    return struct.pack("<8I", 0xFEEDFACF, cpu, 0, 2, 0, 0, 0, 0)


class VSIXAssemblyTests(unittest.TestCase):
    def setUp(self):
        owned = tempfile.TemporaryDirectory(prefix="automexia-vsix-fixture-")
        self.addCleanup(owned.cleanup)
        self.root = Path(owned.name).resolve()
        self.desktop = self.root / "desktop"
        self.desktop.mkdir()
        self.output = self.root / "test.vsix"
        self.epoch = 1760000000

    @staticmethod
    def write(path, content=b"fictional\n", executable=False):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        if executable:
            path.chmod(0o755)
        return path

    def portable(self, binary, suffix="", executable=False):
        for name in package.RUNTIMES:
            self.write(self.desktop / (name + suffix), binary, executable)
        for name in package.DOCUMENTS:
            self.write(self.desktop / name)
        for path in package.tracked_shell_paths():
            self.write(self.desktop / path)
        return self.write(self.root / (package.COMPANION + suffix), binary, executable)

    def packaged_identity(self, bundle):
        identity = ET.fromstring(bundle.read("extension.vsixmanifest")).find(
            ".//{http://schemas.microsoft.com/developer/vsx-schema/2011}Identity"
        )
        self.assertIsNotNone(identity)
        return identity.attrib

    def test_windows_architecture_and_both_conpty_hosts_are_packaged(self):
        companion = self.portable(pe(0x8664), suffix=".exe")
        conpty = tuple(
            self.write(self.desktop / name, pe(machine))
            for name, machine in (("conpty.dll", 0x8664), ("x64/OpenConsole.exe", 0x8664),
                                  ("arm64/OpenConsole.exe", 0xAA64))
        )
        editor = tuple(
            self.write(self.desktop / package.windows_powershell_editor.MODULE_PATH / name)
            for name in sorted(package.windows_powershell_editor.FILES)
        )
        with (mock.patch.object(package.windows_conpty_runtime, "verify_runtime",
                                return_value=conpty) as verify_conpty,
              mock.patch.object(package.windows_powershell_editor, "verify_runtime",
                                return_value=editor) as verify_editor):
            package.assemble("win32-x64", self.desktop, companion, self.output, self.epoch)
        verify_conpty.assert_called_once_with(self.desktop, "x64")
        verify_editor.assert_called_once_with(self.desktop)
        with zipfile.ZipFile(self.output) as bundle:
            self.assertIsNone(bundle.testzip())
            identity = self.packaged_identity(bundle)
            self.assertEqual(identity["Id"], "automexia-wsl")
            self.assertEqual(identity["TargetPlatform"], "win32-x64")
            root = "extension/runtime/win32-x64"
            for path in conpty:
                relative = path.relative_to(self.desktop).as_posix()
                self.assertEqual(bundle.read(f"{root}/desktop/{relative}"), path.read_bytes())
                self.assertEqual(bundle.read(f"{root}/companion/{relative}"), path.read_bytes())
            self.assertEqual(bundle.read(f"{root}/companion/{package.COMPANION}.exe"), companion.read_bytes())
            self.assertIn(f"{root}/desktop/shell-integration/zsh/session/.zshenv", bundle.namelist())
            self.assertNotIn(f"{root}/companion/{editor[0].relative_to(self.desktop).as_posix()}", bundle.namelist())
            types = ET.fromstring(bundle.read("[Content_Types].xml"))
            override_parts = {item.get("PartName") for item in types.findall(
                f"{{{package.CONTENT_NAMESPACE}}}Override"
            )}
            self.assertIn(f"/{root}/desktop/LICENSE", override_parts)
            self.assertIn(f"/{root}/desktop/shell-integration/zsh/session/.zshenv", override_parts)

    @unittest.skipIf(os.name == "nt", "Unix executable modes require a Unix staging filesystem")
    def test_linux_assembly_is_reproducible_with_executable_modes(self):
        companion = self.portable(elf(183), executable=True)
        duplicate = self.root / "second.vsix"
        for destination in (self.output, duplicate):
            package.assemble("linux-arm64", self.desktop, companion, destination, self.epoch)
        self.assertEqual(self.output.read_bytes(), duplicate.read_bytes())
        with zipfile.ZipFile(self.output) as bundle:
            self.assertEqual(self.packaged_identity(bundle)["TargetPlatform"], "linux-arm64")
            for name in (
                "extension/runtime/linux-arm64/desktop/amx",
                f"extension/runtime/linux-arm64/companion/{package.COMPANION}",
            ):
                self.assertEqual(stat.S_IMODE(bundle.getinfo(name).external_attr >> 16), 0o755)
            self.assertIn(b"/extension/runtime/linux-arm64/desktop/amx", bundle.read("[Content_Types].xml"))

    def test_wrong_architecture_and_unexpected_staging_leave_no_output(self):
        companion = self.portable(pe(0x8664), suffix=".exe")
        with self.assertRaisesRegex(ValueError, "architecture differs"):
            package.assemble("win32-arm64", self.desktop, companion, self.output, self.epoch)
        self.assertFalse(self.output.exists())
        self.write(self.desktop / ".unexpected")
        with (mock.patch.object(package.windows_conpty_runtime, "verify_runtime", return_value=()),
              mock.patch.object(package.windows_powershell_editor, "verify_runtime", return_value=())):
            with self.assertRaisesRegex(ValueError, "unexpected entry"):
                package.assemble("win32-x64", self.desktop, companion, self.output, self.epoch)
        self.assertFalse(self.output.exists())

    def test_existing_output_is_preserved(self):
        output = self.write(self.output)
        companion = self.portable(pe(0x8664), suffix=".exe")
        with self.assertRaisesRegex(ValueError, "must be a new"):
            package.assemble("win32-x64", self.desktop, companion, output, self.epoch)
        self.assertEqual(output.read_bytes(), b"fictional\n")

    def test_output_cannot_alias_a_source_tree_through_parent_components(self):
        companion = self.portable(pe(0x8664), suffix=".exe")
        other = self.root / "other"
        other.mkdir()
        output = other / ".." / "desktop" / "test.vsix"
        with self.assertRaisesRegex(ValueError, "outside the source trees"):
            package.assemble("win32-x64", self.desktop, companion, output, self.epoch)
        self.assertFalse(output.exists())

    def test_mach_o_universal_checks_slice_headers_and_bounds(self):
        x64, arm64 = 0x01000007, 0x0100000C
        header = struct.pack(">II", 0xCAFEBABE, 2)
        first = struct.pack(">IIIII", x64, 0, 48, 32, 0)
        last = struct.pack(">IIIII", arm64, 0, 80, 32, 0)
        image = header + first + last + mach(x64) + mach(arm64)
        self.assertEqual(package.mach_architectures(io.BytesIO(image), len(image)), {x64, arm64})
        with self.assertRaisesRegex(ValueError, "slice disagrees"):
            invalid = header + first + last + mach(arm64) + mach(x64)
            package.mach_architectures(io.BytesIO(invalid), len(invalid))
        with self.assertRaisesRegex(ValueError, "extends beyond"):
            package.mach_architectures(io.BytesIO(image[:-1]), len(image) - 1)


if __name__ == "__main__":
    unittest.main()
