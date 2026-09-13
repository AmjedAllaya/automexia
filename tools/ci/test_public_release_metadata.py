#!/usr/bin/env python3
"""Mutations for the public archive's read-only documentation gate."""

import hashlib
import contextlib
import io
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_public_release_metadata as gate


class PublicMetadataTests(unittest.TestCase):
    def test_complete_inventory_gate_rejects_each_missing_owner_and_wrong_key(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in gate.FILES:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('<img src="assets/automexia-logo.png" alt="Automexia logo">\n', encoding="utf-8")
            (root / "README.md").write_text(
                '<img src="assets/automexia-logo.png" alt="Automexia logo">\n' +
                '\n'.join(f'[Download]({gate.RELEASE_ROOT}/{name})' for name in gate.PACKAGES), encoding="utf-8")
            (root / "VERIFY.md").write_text(
                '<img src="assets/automexia-logo.png" alt="Automexia logo">\n' +
                f"minisign -P '{gate.PUBLIC_KEY}'\nsha256sum --check", encoding="utf-8")
            (root / "assets/README.md").write_text("# Brand\n", encoding="utf-8")
            (root / "LICENSE").write_text(gate.read_text(Path(__file__).resolve().parents[2] / "LICENSE"), encoding="utf-8")
            for name in (".gitignore", ".github/CODEOWNERS"):
                (root / name).write_text("", encoding="utf-8")
            (root / "assets/automexia-logo.png").write_bytes(b"fixture mark")
            listing = mock.Mock(stdout=('\0'.join(sorted(gate.FILES))+'\0').encode())
            # Only the public Git inventory and fixture logo digest are supplied;
            # real files, Markdown links and key checks remain independent.
            with mock.patch.object(gate.subprocess, 'run', return_value=listing), \
                    mock.patch.object(gate, 'LOGO_SHA', hashlib.sha256(b"fixture mark").hexdigest()):
                gate.check(root)
                for name in gate.FILES:
                    listing.stdout=('\0'.join(sorted(gate.FILES-{name}))+'\0').encode()
                    with self.subTest(missing=name), self.assertRaises(ValueError):
                        gate.check(root)
                listing.stdout=('\0'.join(sorted(gate.FILES | {'unowned.py'}))+'\0').encode()
                with self.assertRaises(ValueError):
                    gate.check(root)
                listing.stdout=('\0'.join(sorted(gate.FILES))+'\0').encode()
                verify=root / 'VERIFY.md'
                verify.write_text(verify.read_text().replace(gate.PUBLIC_KEY, 'wrong-key'), encoding='utf-8')
                with self.assertRaises(ValueError):
                    gate.check(root)

    def test_cli_errors_do_not_echo_private_root_or_contents(self):
        output=io.StringIO()
        with mock.patch('sys.argv', ['check', '--root', 'fixture-root']), \
                mock.patch.object(gate, 'check', side_effect=ValueError('sensitive-canary')), \
                contextlib.redirect_stdout(output):
            self.assertEqual(gate.main(), 1)
        self.assertNotIn('sensitive-canary', output.getvalue())
        self.assertNotIn('fixture-root', output.getvalue())

    def test_local_links_and_anchors_are_resolved_without_network_or_writes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "guide.md").write_text("# First launch\n", encoding="utf-8")
            before = (root / "guide.md").read_bytes()
            gate.check_links(root, "README.md", '[Start](guide.md#first-launch)')
            self.assertEqual((root / "guide.md").read_bytes(), before)
            for target in ("gone.md", "guide.md#gone", "../outside.md", "/private", "javascript:alert(1)"):
                with self.subTest(target=target), self.assertRaises(ValueError):
                    gate.check_links(root, "README.md", f'[Start]({target})')

    def test_unsafe_links_images_and_private_markers_fail_closed(self):
        gate.check_text("README.md", '[Project](https://example.invalid/)')
        gate.check_text("README.md", '<img src="logo.png" alt="Automexia logo">')
        cases = (
            'https://github.com/example/archive/releases/latest/download/file',
            'http://example.invalid/file',
            'https://user:password@example.invalid/file',
            '[Contact](mailto:someone@example.invalid)',
            '<img src="logo.png">',
            'BEGIN RSA ' + 'PRIVATE KEY',
            'C:' + '/Users/alice/private',
        )
        for content in cases:
            with self.subTest(content=content), self.assertRaises(ValueError):
                gate.check_text("README.md", content)

    def test_brand_bytes_are_exact_and_missing_files_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            asset = root / "logo.png"
            asset.write_bytes(b"original")
            digest = hashlib.sha256(b"original").hexdigest()
            gate.check_asset(asset, digest)
            asset.write_bytes(b"originaL")
            with self.assertRaises(ValueError):
                gate.check_asset(asset, digest)
            with self.assertRaises(ValueError):
                gate.check_asset(root / "missing.png", digest)

    def test_oversized_and_linked_files_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            document = root / "guide.md"
            document.write_bytes(b"x" * (gate.MAX_TEXT + 1))
            with self.assertRaises(ValueError):
                gate.read_text(document)
            link = root / "linked.md"
            try:
                link.symlink_to(document)
            except OSError:
                self.skipTest("native symbolic-link creation is unavailable")
            with self.assertRaises(ValueError):
                gate.read_text(link)
            with self.assertRaises(ValueError):
                gate.check_asset(link, hashlib.sha256(document.read_bytes()).hexdigest())

    def test_all_six_asset_links_are_exact_not_partial_filename_matches(self):
        links = "\n".join(f"[Download]({gate.RELEASE_ROOT}/{name})" for name in gate.PACKAGES)
        gate.check_downloads(links)
        for name in gate.PACKAGES:
            with self.subTest(name=name), self.assertRaises(ValueError):
                gate.check_downloads(links.replace(name, "unowned-" + name))
        with self.assertRaises(ValueError):
            gate.check_downloads(links.replace("v0.4.0", "v9.9.9"))


if __name__ == "__main__":
    unittest.main()
