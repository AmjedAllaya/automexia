"""Pinned editor supply-chain, archive and package-layout regressions."""
import contextlib
import copy
import hashlib
import json
import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET
import zipfile

import windows_powershell_editor as EDITOR


class EditorTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.recipe = copy.deepcopy(EDITOR.load_recipe())
        self.payloads = {name: ('verified-fixture:' + name).encode() for name in EDITOR.FILES}
        for item in self.recipe['files']:
            item['size'] = len(self.payloads[item['path']])
            item['sha256'] = hashlib.sha256(self.payloads[item['path']]).hexdigest()

    def archive(self, extra=None):
        target = self.root / 'editor.nupkg'
        with zipfile.ZipFile(target, 'w') as output:
            for name, data in self.payloads.items():
                output.writestr(name, data)
            if extra is not None:
                output.writestr(*extra)
        self.recipe['package']['sha256'] = hashlib.sha256(target.read_bytes()).hexdigest()
        return target

    def installed(self):
        destination = self.root / 'installed'
        for name, data in self.payloads.items():
            path = destination / EDITOR.MODULE_PATH / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        return destination

    def test_recipe_pins_exact_version_publisher_license_and_eight_files(self):
        recipe = EDITOR.load_recipe()
        self.assertEqual(recipe['package']['version'], '2.4.5')
        self.assertEqual(recipe['package']['publisher'], 'Microsoft Corporation')
        self.assertEqual(recipe['package']['license'], 'MIT')
        self.assertEqual(len(recipe['files']), 8)
        self.assertEqual(sum(item['signed'] for item in recipe['files']), 7)

    def test_recipe_rejects_wrong_identity_missing_duplicate_and_unsafe_files(self):
        mutations = []
        for key, value in [('version', '0.0.0'), ('url', 'https://example.invalid/editor'), ('publisher', 'Unknown'), ('license', 'Unknown')]:
            recipe = copy.deepcopy(self.recipe)
            recipe['package'][key] = value
            mutations.append(recipe)
        recipe = copy.deepcopy(self.recipe)
        recipe['files'][-1] = recipe['files'][0]
        mutations.append(recipe)
        recipe = copy.deepcopy(self.recipe)
        recipe['files'][0]['path'] = '../escape'
        mutations.append(recipe)
        recipe = copy.deepcopy(self.recipe)
        recipe['files'][0]['size'] = EDITOR.MAX_ARCHIVE + 1
        mutations.append(recipe)
        for index, recipe in enumerate(mutations):
            with self.subTest(index=index):
                path = self.root / 'recipe.json'
                path.write_text(json.dumps(recipe))
                with patch.object(EDITOR, 'RECIPE', path), self.assertRaises(EDITOR.EditorError):
                    EDITOR.load_recipe()

    def test_pinned_archive_extracts_only_required_exact_bytes(self):
        archive = self.archive(('unused.txt', b'not installed'))
        destination = self.root / 'members'
        EDITOR.extract_archive(archive, destination, self.recipe)
        self.assertFalse((destination / 'unused.txt').exists())
        for name, expected in self.payloads.items():
            self.assertEqual((destination / name).read_bytes(), expected)

    def test_archive_hash_corruption_is_rejected_before_extraction(self):
        archive = self.archive()
        archive.write_bytes(archive.read_bytes() + b'changed')
        destination = self.root / 'members'
        with self.assertRaises(EDITOR.EditorError):
            EDITOR.extract_archive(archive, destination, self.recipe)
        self.assertFalse(destination.exists())

    def test_archive_rejects_traversal_duplicate_and_link_members(self):
        link = zipfile.ZipInfo('link')
        link.create_system = 3
        link.external_attr = (stat.S_IFLNK | 0o777) << 16
        for index, extra in enumerate([('../escape', b'x'), ('PSREADLINE.PSD1', b'x'), (link, b'../escape')]):
            with self.subTest(index=index):
                archive = self.archive(extra)
                with self.assertRaises(EDITOR.EditorError):
                    EDITOR.extract_archive(archive, self.root / f'members-{index}', self.recipe)
        self.assertFalse((self.root / 'escape').exists())

    def test_archive_member_hash_corruption_is_rejected(self):
        self.payloads['License.txt'] = b'wrong license bytes'
        archive = self.archive()
        with self.assertRaises(EDITOR.EditorError):
            EDITOR.extract_archive(archive, self.root / 'members', self.recipe)

    def test_runtime_verification_rejects_missing_changed_and_extra_files(self):
        destination = self.installed()
        with patch.object(EDITOR, 'load_recipe', return_value=self.recipe):
            self.assertEqual(len(EDITOR.verify_runtime(destination)), 8)
            license_path = destination / EDITOR.MODULE_PATH / 'License.txt'
            license_path.write_bytes(b'changed')
            with self.assertRaises(ValueError):
                EDITOR.verify_runtime(destination)
            license_path.write_bytes(self.payloads['License.txt'])
            extra = destination / EDITOR.MODULE_PATH / 'unexpected.dll'
            extra.write_bytes(b'unknown')
            with self.assertRaises(EDITOR.EditorError):
                EDITOR.verify_runtime(destination)

    @unittest.skipUnless(os.name == 'nt', 'native Windows publication semantics')
    def test_native_preparation_is_idempotent_and_preserves_conflicting_module(self):
        archive = self.archive()
        destination = self.root / 'build'
        with patch.object(EDITOR, 'load_recipe', return_value=self.recipe), \
                patch.object(EDITOR.dev_cache, 'cache_lease', side_effect=lambda *a, **k: contextlib.nullcontext()), \
                patch.object(EDITOR.dev_cache, 'downloads_root', return_value=self.root / 'downloads'), \
                patch.object(EDITOR.dev_cache, 'staging_root', return_value=self.root / 'staging'):
            first = EDITOR.prepare_runtime(destination, archive=archive)
            self.assertEqual(first, EDITOR.prepare_runtime(destination, offline=True))
            marker = destination / EDITOR.MODULE_PATH / 'License.txt'
            marker.write_bytes(b'preserve conflicting data')
            with self.assertRaises(ValueError):
                EDITOR.prepare_runtime(destination, archive=archive)
            self.assertEqual(marker.read_bytes(), b'preserve conflicting data')

    def test_both_installers_include_each_pinned_file_and_referenced_component(self):
        for name in ('automexia.wxs', 'automexia-arm64.wxs'):
            with self.subTest(installer=name):
                tree = ET.parse(EDITOR.ROOT / 'packaging/windows' / name)
                files = [node for node in tree.iter() if node.tag.rsplit('}', 1)[-1] == 'File'
                         and '\\powershell-modules\\' in node.get('Source', '')]
                self.assertEqual(len(files), 8)
                observed = {node.attrib['Source'].split('\\PSReadLine\\2.4.5\\', 1)[1].replace('\\', '/') for node in files}
                self.assertEqual(observed, EDITOR.FILES)
                refs = {node.get('Id') for node in tree.iter() if node.tag.rsplit('}', 1)[-1] == 'ComponentRef'}
                for component in tree.iter():
                    if component.tag.rsplit('}', 1)[-1] == 'Component' and any(node in files for node in component):
                        self.assertIn(component.attrib['Id'], refs)
                        members = [node for node in component if node.tag.rsplit('}', 1)[-1] == 'File']
                        self.assertEqual(len(members), 1, 'each editor file must be independently repairable')
                        self.assertEqual(members[0].get('KeyPath'), 'yes')


if __name__ == '__main__':
    unittest.main()
