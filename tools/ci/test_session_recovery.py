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


if __name__ == "__main__":
    unittest.main()
