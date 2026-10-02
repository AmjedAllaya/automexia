"""Generic discovery must not fabricate mandatory native fixture evidence."""
from pathlib import Path
import unittest
from unittest.mock import patch

import test_ssh_helper_powershell_transfer as windows
import test_ssh_helper_runtime as helper
import test_ssh_helper_transfer as posix
import test_ssh_helper_transfer_runtime as transfer


class FixtureDiscoveryTests(unittest.TestCase):
    def test_unconfigured_classes_skip_before_allocating_or_launching(self):
        cases = (
            (windows, windows.WindowsHelperTransferTests,
             dict(POWERSHELL='', EXPORTER='', SHELL='', HELPER='')),
            (posix, posix.PosixHelperTransferTests, dict(EXPORTER='')),
            (helper, helper.HelperRuntimeContracts, dict(IMAGE='', APPLICATION=None, HELPER=None)),
            (transfer, transfer.HelperTransferRuntimeTests, dict(IMAGE='', EXPORTER='')),
        )
        for module, case, inputs in cases:
            with self.subTest(module=module.__name__), patch.multiple(module, **inputs), \
                    patch('tempfile.TemporaryDirectory', side_effect=AssertionError('unexpected allocation')), \
                    patch.object(helper.runtime, 'Fixture', side_effect=AssertionError('unexpected server')):
                with self.assertRaises(unittest.SkipTest):
                    case.setUpClass()

    def test_partial_configuration_is_an_error_not_a_skip(self):
        cases = (
            (windows, windows.WindowsHelperTransferTests,
             dict(POWERSHELL='fixture', EXPORTER='', SHELL='', HELPER='')),
            (helper, helper.HelperRuntimeContracts, dict(IMAGE='fixture', APPLICATION=None, HELPER=None)),
            (transfer, transfer.HelperTransferRuntimeTests, dict(IMAGE='fixture', EXPORTER='')),
        )
        for module, case, inputs in cases:
            with self.subTest(module=module.__name__), patch.multiple(module, **inputs):
                with self.assertRaises(ValueError):
                    case.setUpClass()

    def test_explicit_native_configuration_never_skips_wrong_platform(self):
        with patch.multiple(windows, POWERSHELL='fixture', EXPORTER='fixture', SHELL='powershell', HELPER='fixture'), \
                patch.object(windows.os, 'name', 'posix'):
            with self.assertRaises(RuntimeError):
                windows.WindowsHelperTransferTests.setUpClass()
        with patch.object(posix, 'EXPORTER', 'fixture'), patch.object(posix.os, 'name', 'nt'):
            with self.assertRaises(RuntimeError):
                posix.PosixHelperTransferTests.setUpClass()
        for module, case, inputs in (
            (helper, helper.HelperRuntimeContracts, dict(IMAGE='fixture', APPLICATION=Path('app'), HELPER=Path('helper'))),
            (transfer, transfer.HelperTransferRuntimeTests, dict(IMAGE='fixture', EXPORTER='fixture')),
        ):
            with self.subTest(module=module.__name__), patch.multiple(module, **inputs), patch.object(module.sys, 'platform', 'win32'):
                with self.assertRaises(RuntimeError):
                    case.setUpClass()

    def test_configured_runtime_keeps_invalid_image_as_a_failure(self):
        with patch.multiple(helper, IMAGE='not-an-immutable-image', APPLICATION=Path('app'), HELPER=Path('helper')), \
                patch.object(helper.sys, 'platform', 'linux'):
            with self.assertRaises(ValueError):
                helper.HelperRuntimeContracts.setUpClass()


if __name__ == '__main__':
    unittest.main()
