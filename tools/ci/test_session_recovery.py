#!/usr/bin/env python3
"""Mutation canaries for the recovery architecture contract."""
import json
import shutil
import subprocess
import sys
import unittest
import check_session_recovery as policy


class RecoveryNativeOracleTests(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "win32", "Native DWM window ownership requires Windows")
    def test_native_locator_waits_for_an_uncloaked_application_window(self):
        script = r'''
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -Path 'tests/integration/windows-native-window-locator.cs'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class RecoveryLocatorFixture {
    [DllImport("dwmapi.dll")]
    public static extern int DwmSetWindowAttribute(IntPtr window, uint attribute, ref int value, uint size);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindowVisible(IntPtr window);
}
'@
$form = [Windows.Forms.Form]::new()
$form.Text = 'Recovery window locator fixture'
$form.ClientSize = [Drawing.Size]::new(320, 200)
$form.ShowInTaskbar = $false
try {
    $window = $form.Handle
    $cloak = 1
    if ([RecoveryLocatorFixture]::DwmSetWindowAttribute($window, 13, [ref]$cloak, 4) -ne 0) {
        throw 'Could not cloak the owned fixture window'
    }
    $form.Show()
    [Windows.Forms.Application]::DoEvents()
    $styleVisible = [RecoveryLocatorFixture]::IsWindowVisible($window)
    $duringStartup = @([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($PID)) -contains $window
    $cloak = 0
    if ([RecoveryLocatorFixture]::DwmSetWindowAttribute($window, 13, [ref]$cloak, 4) -ne 0) {
        throw 'Could not reveal the owned fixture window'
    }
    [Windows.Forms.Application]::DoEvents()
    $presented = @([AutomexiaNativeWindowLocator]::VisibleApplicationWindows($PID)) -contains $window
    @{ style_visible = $styleVisible; during_startup = $duringStartup; presented = $presented } | ConvertTo-Json -Compress
} finally { $form.Close(); $form.Dispose() }
'''
        result = subprocess.run(
            ["powershell.exe", "-NoLogo", "-NoProfile", "-NonInteractive", "-STA", "-Command", script],
            cwd=policy.ROOT, capture_output=True, text=True, timeout=30, check=True,
        )
        self.assertEqual(json.loads(result.stdout), {
            "style_visible": True, "during_startup": False, "presented": True,
        })

    def test_history_sentinel_respects_soft_wrap_and_session_boundaries(self):
        shell = shutil.which("powershell.exe") or shutil.which("pwsh")
        if not shell:
            self.skipTest("PowerShell is required for the native recovery oracle")
        script = r'''
$ErrorActionPreference = 'Stop'
# Get-Location can carry a provider-qualified UNC path under WSL. The .NET
# parser requires a filesystem path, not PowerShell's provider syntax.
$source = (Get-Item -LiteralPath 'tests/integration/session-recovery-windows.ps1').FullName
$parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile(
    $source, [ref]$null, [ref]$parseErrors)
if ($parseErrors.Count -gt 0) { throw 'Recovery fixture could not be parsed' }
$function = $ast.Find({ param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst] -and
    $node.Name -eq 'Get-RecoverySentinelCount'
}, $true)
if ($null -eq $function) { throw 'Recovery sentinel oracle is missing' }
. ([scriptblock]::Create($function.Extent.Text))
function Row($text, $wrap) {
    $bytes = [Collections.Generic.List[byte]]::new()
    foreach ($character in $text.ToCharArray()) {
        $bytes.AddRange([BitConverter]::GetBytes([uint64][char]$character))
    }
    return @{ cells = [Convert]::ToBase64String($bytes.ToArray()); wrap = $wrap }
}
function Session($rows) { return @{ history = @{ rows = @($rows) } } }
function Count($sessions) {
    return Get-RecoverySentinelCount @{ windows = @(@{ tabs = @(@{
        nodes = @(@{ sessions = @($sessions) })
    }) }) }
}
@{
    wrapped = Count @((Session @((Row 'RECOVERY_HISTORY_' $true), (Row 'SENTINEL' $false))))
    hard_break = Count @((Session @((Row 'RECOVERY_HISTORY_' $false), (Row 'SENTINEL' $false))))
    separate_sessions = Count @((Session @((Row 'RECOVERY_HISTORY_' $true))), (Session @((Row 'SENTINEL' $false))))
    command_and_output = Count @((Session @((Row 'Write-Output RECOVERY_HISTORY_' $true), (Row 'SENTINEL' $false), (Row 'RECOVERY_HISTORY_SENTINEL' $false))))
} | ConvertTo-Json -Compress
'''
        result = subprocess.run(
            [shell, "-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script],
            cwd=policy.ROOT, capture_output=True, text=True, timeout=30, check=True,
        )
        self.assertEqual(json.loads(result.stdout), {
            "wrapped": 1, "hard_break": 0, "separate_sessions": 0,
            "command_and_output": 2,
        })


class RecoveryArchitectureTests(unittest.TestCase):
    def setUp(self):
        self.sources = {p: (policy.ROOT / p).read_text(encoding="utf-8") for p in policy.FILES}

    def test_current_contract(self):
        policy.validate(self.sources)

    def test_shared_protection_dependencies_are_available_without_test_or_windows_cfg(self):
        manifest = (policy.ROOT / "apps/automexia-terminal/Cargo.toml").read_text(encoding="utf-8")
        policy.validate_dependencies(manifest)

    def test_target_only_or_optional_shared_dependencies_are_rejected(self):
        manifest = (policy.ROOT / "apps/automexia-terminal/Cargo.toml").read_text(encoding="utf-8")
        declaration = 'base64 = { workspace = true }'
        for section in ("[dev-dependencies]", "[target.'cfg(windows)'.dependencies]"):
            moved = manifest.replace(declaration, "").replace(section, section + "\n" + declaration)
            with self.subTest(section=section), self.assertRaisesRegex(AssertionError, "Shared recovery dependency base64"):
                policy.validate_dependencies(moved)
        optional = manifest.replace(declaration, 'base64 = { workspace = true, optional = true }')
        with self.assertRaisesRegex(AssertionError, "must not be optional"):
            policy.validate_dependencies(optional)

    def test_persisted_environment_is_rejected(self):
        path = policy.FILES[0]
        self.sources[path] = self.sources[path].replace("pub struct Session {", "pub struct Session {\n    pub environment: Vec<String>,")
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_launch_from_storage_is_rejected(self):
        self.sources[policy.FILES[1]] += '\nCommand::new("ssh");\n'
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_removed_consent_is_rejected(self):
        path = policy.FILES[4]
        self.sources[path] = self.sources[path].replace("take_recovery_choice", "bypass_choice")
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_removed_private_lock_is_rejected(self):
        path = policy.FILES[1]
        self.sources[path] = self.sources[path].replace("WriteLock::try_acquire", "no_lock")
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_unix_restore_cannot_ignore_its_working_directory(self):
        path = policy.FILES[3]
        self.sources[path] = self.sources[path].replace("config.use_fork = false", "config.use_fork = true")
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_serializable_live_capture_is_rejected(self):
        path = policy.FILES[0]
        self.sources[path] = self.sources[path].replace("#[serde(skip)]", "")
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_plaintext_storage_is_rejected(self):
        path = policy.FILES[1]
        self.sources[path] = self.sources[path].replace("protection.borrow_mut().seal", "plaintext")
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_machine_wide_key_scope_is_rejected(self):
        self.sources[policy.FILES[7]] += "\nCRYPTPROTECT_LOCAL_MACHINE"
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_macos_dialog_policy_cannot_be_removed(self):
        path = policy.FILES[9]
        self.sources[path] = self.sources[path].replace("allowed == 0", "true")
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_macos_key_overwrite_is_rejected(self):
        self.sources[policy.FILES[9]] += "\nSecItemUpdate"
        with self.assertRaises(AssertionError):
            policy.validate(self.sources)

    def test_current_reviewed_dependency(self):
        policy.validate_vendor(*policy.vendor_inputs())

    def test_unreviewed_dependency_edits_are_rejected(self):
        files, *inputs = policy.vendor_inputs()
        files["src/reserved.rs"] += "\n// unreviewed change\n"
        with self.assertRaises(AssertionError):
            policy.validate_vendor(files, *inputs)

    def test_extra_dependency_file_is_rejected(self):
        files, *inputs = policy.vendor_inputs()
        files["build.rs"] = "fn main() {}"
        with self.assertRaises(AssertionError):
            policy.validate_vendor(files, *inputs)

    def test_registry_dependency_substitution_is_rejected(self):
        files, manifest, lock, config = policy.vendor_inputs()
        manifest = manifest.replace('inout = { path = "third-party/inout" }', '')
        with self.assertRaises((AssertionError, KeyError)):
            policy.validate_vendor(files, manifest, lock, config)


if __name__ == "__main__":
    unittest.main()
