"""Check download selection, publication arguments and release-note preservation."""
import re
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import yaml

import release_downloads as downloads


ROOT = Path(__file__).resolve().parents[2]


class ReleaseDownloadsTests(unittest.TestCase):
    def test_guide_links_cover_exactly_the_published_packages(self):
        for version, tag in (("0.0.1-alpha", "0.0.1_alpha"), ("1.2.3", "v1.2.3")):
            guide = downloads.download_guide(version)
            links = re.findall(r"\]\((https://[^)]+)\)", guide)
            base = f"https://github.com/AmjedAllaya/automexia/releases/download/{tag}/"
            self.assertEqual(set(links), {base + name for name in downloads.package_names(version).values()})
            self.assertEqual(len(links), 11)
            for term in ("Intel", "Apple Silicon", "Settings > System > About", "uname -m",
                         "Debian", "Ubuntu", "Linux Mint", "Fedora", "openSUSE", "glibc) 2.35",
                         "Alpine/musl", "Source code", "SHA256SUMS", "without installing"):
                self.assertIn(term, guide)

    def test_note_replacement_retains_signing_and_limitations_verbatim(self):
        source = (ROOT / "docs/public-release/FIRST-ALPHA-NOTES.md").read_text(encoding="utf-8")
        result = downloads.replace_download_guide(source, "0.0.1-alpha")
        self.assertEqual(result.split("## Signing and verification", 1)[1],
                         source.split("## Signing and verification", 1)[1])
        self.assertEqual(result.split("## Choose your download", 1)[0],
                         source.split("## Choose your download", 1)[0])
        self.assertEqual(result.count("## Choose your download"), 1)
        self.assertEqual(result, downloads.replace_download_guide(result, "0.0.1-alpha"))
        with self.assertRaises(ValueError):
            downloads.replace_download_guide("Missing template section", "0.0.1-alpha")

    def test_upload_cli_emits_labels_without_changing_files_and_rejects_extras(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            package = root / "automexia-1.2.3-windows-x64-installer.msi"
            package.write_bytes(b"signed package fixture")
            command = [sys.executable, str(ROOT / "tools/ci/release_downloads.py"),
                       "upload-arguments", "--version", "1.2.3", "--directory", str(root)]
            result = subprocess.run(command, capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), str(package) + "#Windows - Intel/AMD 64-bit (x64) - Installer (.msi)")
            self.assertEqual(package.read_bytes(), b"signed package fixture")
            (root / "unreviewed.zip").write_bytes(b"not a release asset")
            result = subprocess.run(command, capture_output=True, text=True, timeout=15)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")

    def test_stable_publisher_generates_guide_and_labels_before_publication(self):
        workflow = yaml.safe_load((ROOT / ".github/workflows/release.yml").read_text())
        script = next(step["run"] for step in workflow["jobs"]["publish-release"]["steps"]
                      if "gh release create" in step.get("run", ""))
        self.assertLess(script.index("release_downloads.py guide"), script.index("gh release create"))
        self.assertIn("--notes-file download-guide.md", script)
        self.assertLess(script.index("release_downloads.py upload-arguments"), script.index("gh release upload"))
        self.assertIn('"${labelled_assets[@]}"', script)
        self.assertIn('gh release verify-asset "$tag" "$asset"', script)


if __name__ == "__main__":
    unittest.main()
