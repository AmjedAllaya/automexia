"""Keep candidate replays bound to source and the actual release gate."""

import copy
import unittest

from windows_candidate_review import release_scripts, validate_source


class CandidateReviewTests(unittest.TestCase):
    def test_source_rejects_wrong_identity(self):
        commit = "a" * 40
        run = {"head_sha": commit, "head_branch": "main", "event": "workflow_dispatch",
               "path": ".github/workflows/first-alpha.yml",
               "repository": {"full_name": "example/terminal"}}
        validate_source(run, "example/terminal", commit)
        for field, value in (("head_sha", "b" * 40), ("head_branch", "other"),
                             ("event", "pull_request"), ("path", "other.yml"),
                             ("repository", {"full_name": "other/repo"})):
            changed = copy.deepcopy(run)
            changed[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate_source(changed, "example/terminal", commit)

    def test_replay_keeps_install_and_all_five_serial_gui_cases(self):
        install, gui = release_scripts()
        self.assertIn("Assert-ConPtyRuntime", install)
        self.assertIn("msiexec.exe", install)
        self.assertIn("foreach ($kind in @('zip', 'msi'))", gui)
        self.assertIn("foreach ($policy in @('Restricted', 'RemoteSigned'))", gui)
        self.assertIn("-Policy RemoteSigned -InternetMark", gui)
        self.assertEqual(gui.count("-CandidateCommit $env:CANDIDATE_COMMIT"), 2)
        self.assertNotIn("${{", install + gui)


if __name__ == "__main__":
    unittest.main()
