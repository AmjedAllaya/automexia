"""Keep native Windows PowerShell probes separate from build-shell modules."""
import os
from pathlib import Path
import unittest

import check_windows_editor_resize as CHECK


class WindowsEditorEnvironmentTests(unittest.TestCase):
    def test_native_defaults_do_not_inherit_core_module_paths(self):
        parent = {'PSMODULEPATH': 'core modules', 'PATH': 'tools'}
        child = CHECK.powershell_environment(parent, None)
        self.assertEqual(child, {'PATH': 'tools'})
        self.assertEqual(parent['PSMODULEPATH'], 'core modules')

    def test_packaged_path_has_one_case_insensitive_key_and_preserves_other_values(self):
        parent = {'PSMODULEPATH': 'core modules', 'pSmOdUlEpAtH': 'another core path',
                  'PATH': 'tools', 'SystemRoot': 'system'}
        native_path = 'packaged modules;native modules'
        child = CHECK.powershell_environment(parent, native_path)
        keys = [key for key in child if key.casefold() == 'psmodulepath']
        self.assertEqual(len(keys), 1)
        self.assertEqual(child[keys[0]], native_path)
        self.assertEqual(child['PATH'], parent['PATH'])
        self.assertEqual(child['SystemRoot'], parent['SystemRoot'])
        self.assertEqual(len(parent), 4)

    @unittest.skipUnless(os.name == 'nt', 'native Windows PowerShell module resolution')
    def test_native_default_security_command_resolves_without_parent_module_paths(self):
        environment = dict(os.environ, PSMODULEPATH='unavailable-core-only-modules')
        executable = Path(os.environ['SystemRoot']) / 'System32/WindowsPowerShell/v1.0/powershell.exe'
        output = bytearray()
        def consume(chunk):
            if len(output) + len(chunk) > 4096:
                raise ValueError('Native module control exceeds its output bound')
            output.extend(chunk)
        result = CHECK.qa_process.run(
            [str(executable), '-NoProfile', '-NonInteractive', '-Command',
             '(Get-Command Get-AuthenticodeSignature -ErrorAction Stop).Name'],
            cwd=CHECK.ROOT, environment=CHECK.powershell_environment(environment, None),
            timeout_seconds=30, consume=consume,
        )
        self.assertEqual(result.return_code, 0)
        self.assertFalse(result.timed_out)
        self.assertIsNone(result.error)
        self.assertEqual(bytes(output).strip(), b'Get-AuthenticodeSignature')


if __name__ == '__main__':
    unittest.main()
