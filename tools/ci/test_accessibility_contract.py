#!/usr/bin/env python3
"""Mutation tests for actual native dependency provenance and ownership gates."""
from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import shutil
import tempfile
import unittest

import check_accessibility_contract as contract


class AccessibilityContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline = contract.inputs()
        contract.validate(*cls.baseline)

    def rejected(self, arguments):
        with self.assertRaises((ValueError, KeyError)):
            contract.validate(*arguments)

    def test_checked_in_baseline(self):
        contract.validate(*self.baseline)

    def test_each_changed_removed_or_unreviewed_file_is_rejected(self):
        for name in self.baseline[0]:
            with self.subTest(file=name):
                changed = list(deepcopy(self.baseline))
                changed[0][name] += '\n// unreviewed modification\n'
                self.rejected(changed)
                del changed[0][name]
                self.rejected(changed)
        added = list(deepcopy(self.baseline))
        added[0]['src/unreviewed.rs'] = '// new source'
        self.rejected(added)

    def test_registry_substitution_or_removed_override_is_rejected(self):
        changed = list(deepcopy(self.baseline))
        needle = 'accesskit_windows = { path = "third-party/accesskit-windows" }'
        self.assertIn(needle, changed[1])
        changed[1] = changed[1].replace(needle, '')
        self.rejected(changed)
        changed = list(deepcopy(self.baseline))
        needle = 'name = "accesskit_windows"\nversion = "0.35.1"'
        self.assertIn(needle, changed[2])
        changed[2] = changed[2].replace(needle, needle + '\nsource = "registry+https://github.com/rust-lang/crates.io-index"')
        self.rejected(changed)

    def test_local_source_cannot_claim_registry_audit(self):
        changed = list(deepcopy(self.baseline))
        needle = '[policy.accesskit_windows]\naudit-as-crates-io = false'
        self.assertIn(needle, changed[3])
        changed[3] = changed[3].replace(needle, needle.replace('false', 'true'))
        self.rejected(changed)

    def test_native_adapters_and_assurance_tools_stay_out_of_model_runtime(self):
        for dependency in ['accesskit_windows = "=0.35.1"', 'proptest = { workspace = true }', 'loom = { workspace = true }']:
            with self.subTest(dependency=dependency):
                changed = list(deepcopy(self.baseline))
                changed[4] = changed[4].replace('[dependencies]', '[dependencies]\n' + dependency)
                self.rejected(changed)

    def test_backend_version_drift_is_rejected(self):
        for name, version in [('accesskit_windows', '0.35.1'), ('accesskit_macos', '0.27.1'), ('accesskit_unix', '0.24.0')]:
            changed = list(deepcopy(self.baseline))
            needle = f'{name} = "={version}"'
            self.assertIn(needle, changed[5])
            changed[5] = changed[5].replace(needle, f'{name} = "{version}"')
            self.rejected(changed)

    def test_each_unix_and_macos_source_change_requires_review(self):
        for vendor, files in self.baseline[6].items():
            for name in files:
                with self.subTest(vendor=vendor, file=name):
                    changed = list(deepcopy(self.baseline))
                    changed[6][vendor][name] += '\n// unreviewed\n'
                    self.rejected(changed)
                    del changed[6][vendor][name]
                    self.rejected(changed)
            added = list(deepcopy(self.baseline))
            added[6][vendor]['src/unreviewed.rs'] = '// new source'
            self.rejected(added)

    def test_each_native_fork_requires_local_source_policy_and_override(self):
        for vendor, (package, version, _) in contract.ADDITIONAL_ADAPTERS.items():
            with self.subTest(package=package):
                changed = list(deepcopy(self.baseline))
                needle = f'{package} = {{ path = "{vendor}" }}'
                self.assertIn(needle, changed[1])
                changed[1] = changed[1].replace(needle, '')
                self.rejected(changed)
                changed = list(deepcopy(self.baseline))
                needle = f'name = "{package}"\nversion = "{version}"'
                self.assertIn(needle, changed[2])
                changed[2] = changed[2].replace(needle, needle + '\nsource = "registry+https://github.com/rust-lang/crates.io-index"')
                self.rejected(changed)
                changed = list(deepcopy(self.baseline))
                needle = f'[policy.{package}]\naudit-as-crates-io = false'
                self.assertIn(needle, changed[3])
                changed[3] = changed[3].replace(needle, needle.replace('false', 'true'))
                self.rejected(changed)

    def test_source_reader_rejects_oversized_files_before_reading(self):
        with tempfile.TemporaryDirectory(prefix='automexia-accessibility-contract-') as directory:
            root = Path(directory)
            vendor = root / contract.VENDOR
            shutil.copytree(contract.ROOT / contract.VENDOR, vendor)
            self.assertEqual(contract.bundle(root), self.baseline[0])
            (vendor / 'oversized.rs').write_bytes(b'x' * (contract.MAX_FILE_BYTES + 1))
            with self.assertRaisesRegex(ValueError, 'file exceeds bound'):
                contract.bundle(root)


if __name__ == '__main__':
    unittest.main()
