"""Keep both MSI backends on the same readable Automexia artwork and copy."""

from pathlib import Path
import os
import shutil
import struct
import subprocess
import tempfile
import tomllib
import unittest
import xml.etree.ElementTree as ET

import windows_installer_branding


ROOT = Path(__file__).resolve().parents[2]
BRAND = ROOT / "assets/brand/windows-installer"


class WindowsInstallerBrandingTests(unittest.TestCase):
    def test_compiled_msi_accepts_branding_and_rejects_default_artwork_and_copy(self):
        if os.name != "nt":
            self.skipTest("native MSI inspection requires Windows")
        candle = shutil.which("candle.exe")
        if candle is None:
            cached = Path(os.environ.get("LOCALAPPDATA", "")) / ".cargo-packager/WixTools/candle.exe"
            candle = str(cached) if cached.is_file() else None
        if candle is None:
            if os.environ.get("AUTOMEXIA_REQUIRE_WIX3_BRANDING_TEST") == "1":
                self.fail("WiX 3 is required for this native Windows packaging job")
            self.skipTest("WiX 3 required for native compilation; source checks still run")
        wix = Path(candle).parent
        frontend = ROOT / "apps/automexia-terminal"
        config = tomllib.loads((frontend / "Cargo.toml").read_text(encoding="utf-8"))
        settings = config["package"]["metadata"]["packager"]["wix"]
        with tempfile.TemporaryDirectory(prefix="automexia-branding-") as directory:
            output = Path(directory)
            for mode in ("branded", "default-artwork", "default-copy"):
                obj = output / (mode + ".wixobj")
                msi = output / (mode + ".msi")
                compiler = [candle, "-nologo", "-arch", "x64", "-out", str(obj),
                            "-dBranded=" + ("no" if mode == "default-artwork" else "yes"),
                            "-dIconPath=" + str(ROOT / "assets/brand/automexia-terminal.ico"),
                            "-dBannerPath=" + str((frontend / settings["banner-path"]).resolve()),
                            "-dDialogPath=" + str((frontend / settings["dialog-image-path"]).resolve()),
                            "-dLicensePath=" + str(ROOT / "packaging/windows/license.rtf"),
                            str(ROOT / "tools/ci/fixtures/windows-installer-branding.wxs")]
                linker = [str(wix / "light.exe"), "-nologo", "-ext", str(wix / "WixUIExtension.dll"),
                          "-cultures:en-us", "-out", str(msi), str(obj)]
                if mode != "default-copy":
                    linker.extend(["-loc", str((frontend / settings["languages"][0]["path"]).resolve())])
                for command in (compiler, linker):
                    result = subprocess.run(command, capture_output=True, text=True, timeout=60)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                with self.subTest(mode=mode):
                    if mode == "branded":
                        windows_installer_branding.verify(msi)
                    else:
                        with self.assertRaisesRegex(ValueError, "incorrect"):
                            windows_installer_branding.verify(msi)

    def test_x64_packager_uses_branded_artwork_and_copy(self):
        frontend = ROOT / "apps/automexia-terminal"
        config = tomllib.loads((frontend / "Cargo.toml").read_text(encoding="utf-8"))
        wix = config["package"]["metadata"]["packager"]["wix"]
        for field, name in (("banner-path", "banner.bmp"),
                            ("dialog-image-path", "dialog.bmp")):
            self.assertEqual((frontend / wix[field]).resolve(), BRAND / name)
        self.assertEqual(wix["languages"], [{
            "identifier": "en-US", "path": "../../packaging/windows/automexia-en-us.wxl",
        }])

    def test_arm64_uses_the_same_artwork_and_native_wizard(self):
        document = ET.parse(ROOT / "packaging/windows/automexia-arm64.wxs")
        ns = {"w": "http://wixtoolset.org/schemas/v4/wxs",
              "ui": "http://wixtoolset.org/schemas/v4/wxs/ui"}
        ui = document.find(".//ui:WixUI", ns)
        self.assertIsNotNone(ui, "ARM64 must have a native setup wizard")
        self.assertEqual(ui.attrib, {"Id": "WixUI_InstallDir", "InstallDirectory": "INSTALLFOLDER"})
        variables = {item.attrib["Id"]: item.attrib["Value"]
                     for item in document.findall(".//w:WixVariable", ns)}
        for variable, filename in (("WixUIBannerBmp", "banner.bmp"),
                                   ("WixUIDialogBmp", "dialog.bmp")):
            self.assertEqual(variables[variable], f"$(var.InstallerBrandRoot)\\{filename}")

    def test_bitmap_format_dimensions_and_text_safe_areas(self):
        for name, height, text_edge, text_bottom in (
                ("banner.bmp", 58, 420, 54), ("dialog.bmp", 312, 493, 312)):
            with self.subTest(bitmap=name):
                data = (BRAND / name).read_bytes()
                self.assertEqual(data[:2], b"BM")
                offset = struct.unpack_from("<I", data, 10)[0]
                width, actual_height, planes, depth, compression = struct.unpack_from("<iiHHI", data, 18)
                self.assertEqual((width, actual_height, planes, depth, compression), (493, height, 1, 24, 0))
                stride = (width * 3 + 3) & ~3
                self.assertEqual(len(data), offset + stride * height)
                # Native WiX text must remain on white, separate from decorative artwork.
                left = 170 if name == "dialog.bmp" else 0
                for y in range(text_bottom):
                    row = offset + (height - 1 - y) * stride
                    self.assertEqual(data[row + left * 3:row + text_edge * 3],
                                     b"\xff" * ((text_edge - left) * 3))
                self.assertGreater(len(set(data[offset:])), 64, "logo must not be a flat placeholder")

    def test_wix_versions_share_concise_localized_text(self):
        copies = []
        for name in ("automexia-en-us.wxl", "automexia-arm64-en-us.wxl"):
            document = ET.parse(ROOT / "packaging/windows" / name)
            copies.append({item.attrib["Id"]: item.attrib.get("Value", item.text)
                           for item in document.getroot()})
        self.assertEqual(copies[0], copies[1])
        for key in ("WelcomeDlgDescription", "MaintenanceWelcomeDlgDescription",
                    "ExitDialogDescription", "InstallDirDlgDescription", "UserExitDescription1"):
            self.assertLessEqual(len(copies[0][key]), 170)
        # The same finish dialog also follows removal; do not claim the app was installed.
        self.assertEqual(copies[0]["ExitDialogTitle"], r"{\WixUI_Font_Bigger}Setup complete")

    def test_arm64_license_displays_the_authoritative_license(self):
        rtf = (ROOT / "packaging/windows/license.rtf").read_text(encoding="ascii")
        header = r"{\rtf1\ansi\deff0{\fonttbl{\f0 Segoe UI;}}\fs18 "
        self.assertTrue(rtf.startswith(header))
        self.assertTrue(rtf.endswith("}"))
        text = rtf[len(header):-1].replace(r"\par ", "\n")
        text = text.replace(r"\{", "{").replace(r"\}", "}").replace("\\\\", "\\")
        self.assertEqual(text, (ROOT / "LICENSE").read_text(encoding="ascii"))


if __name__ == "__main__":
    unittest.main()
