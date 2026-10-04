#!/usr/bin/env python3
"""Mutation canaries for the recovery architecture contract."""
import unittest
import check_session_recovery as policy


class RecoveryArchitectureTests(unittest.TestCase):
    def setUp(self):
        self.sources = {p: (policy.ROOT / p).read_text(encoding="utf-8") for p in policy.FILES}

    def test_current_contract(self):
        policy.validate(self.sources)

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
