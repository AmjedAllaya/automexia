"""Regression checks for owner publication without administrative CI credentials."""
import base64
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import subprocess
import unittest
from unittest.mock import patch

import first_alpha_publish as owner

SHA = "a" * 40
RUN_ID = 123


def native_evidence():
    run = {"id": RUN_ID, "head_sha": SHA, "head_branch": "main", "event": "workflow_dispatch",
           "path": ".github/workflows/first-alpha.yml", "repository": {"full_name": owner.REPOSITORY},
           "actor": {"login": owner.REPOSITORY.split("/")[0]}, "status": "completed", "conclusion": "success"}
    names = []
    for name, count in owner.REQUIRED_JOBS.items():
        if name == "Build native release binaries":
            names.extend(name + " (" + target + ")" for target in sorted(owner.alpha.TARGETS))
        else:
            names.extend([name] if count == 1 else [f"{name} ({index})" for index in range(count)])
    names.extend("Native platform accessibility, session and visual QA / " + name for name in sorted(owner.NATIVE_JOBS))
    jobs = [{"id": index + 1, "name": name, "status": "completed", "conclusion": "success"} for index, name in enumerate(names)]
    return run, jobs


def manifest():
    return {"schema": 1, "project": "automexia-terminal", "source_repository": "https://github.com/" + owner.REPOSITORY,
            "version": "0.0.3-alpha", "tag": "v0.0.3-alpha", "source_commit": SHA,
            "channel": "first-alpha", "os_signing": owner.alpha.signing_disclosure(), "branding": {"stable_approval": False}}


def seal(directory):
    data = manifest()
    data["artifacts"] = [{"file": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "size": path.stat().st_size}
                         for path in sorted(directory.iterdir()) if path.name not in ("release-manifest.json", "SHA256SUMS", "SHA256SUMS.minisig")]
    (directory / "release-manifest.json").write_text(json.dumps(data), encoding="utf-8")
    lines = [hashlib.sha256(path.read_bytes()).hexdigest() + "  " + path.name for path in sorted(directory.iterdir()) if path.name not in ("SHA256SUMS", "SHA256SUMS.minisig")]
    (directory / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="ascii")
    (directory / "SHA256SUMS.minisig").write_text("signature verification is mocked only in unit fixtures", encoding="ascii")


def make_bundle(directory):
    names = list(owner.inventory.package_names(owner.alpha.ALPHA_VERSION).values())
    names.extend(owner.inventory.SBOM_NAMES)
    for name in names:
        (directory / name).write_bytes(b"unit fixture")
    seal(directory)


def remote(records, *, draft=True, immutable=False):
    return {"id": 45, "tag_name": "v0.0.3-alpha", "target_commitish": SHA, "prerelease": True, "draft": draft, "immutable": immutable,
            "body": "Alpha fixture notes", "name": "Automexia Terminal v0.0.3-alpha",
            "assets": [{"name": name, "digest": "sha256:" + digest, "size": size, "state": "uploaded"} for name, (digest, size) in records.items()]}


class EvidenceTests(unittest.TestCase):
    def test_all_native_evidence_is_required(self):
        run, jobs = native_evidence()
        owner.validate_run(run, jobs, SHA, RUN_ID)
        for index in range(len(jobs)):
            for state in ("failure", "cancelled", "skipped", ""):
                bad = copy.deepcopy(jobs)
                bad[index]["conclusion"] = state
                with self.assertRaises(ValueError):
                    owner.validate_run(run, bad, SHA, RUN_ID)
            with self.assertRaises(ValueError):
                owner.validate_run(run, jobs[:index] + jobs[index + 1:], SHA, RUN_ID)

    def test_wrong_source_run_repository_actor_or_status_rejected(self):
        run, jobs = native_evidence()
        for key, value in [("id", 999), ("head_sha", "b"*40), ("head_branch", "feature"), ("event", "pull_request"),
                           ("path", "other.yml"), ("repository", {"full_name": "other/project"}), ("actor", {"login": "other"}),
                           ("status", "in_progress"), ("conclusion", "failure")]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                owner.validate_run({**run, key: value}, jobs, SHA, RUN_ID)

    def test_only_original_publication_job_can_be_skipped(self):
        run, jobs = native_evidence()
        skipped = {"id": 999, "name": owner.LEGACY_PUBLISHER, "status": "completed", "conclusion": "skipped"}
        owner.validate_run(run, jobs + [skipped], SHA, RUN_ID)
        for change in ({"name": "Other skipped job"}, {"conclusion": "failure"}):
            with self.assertRaises(ValueError):
                owner.validate_run(run, jobs + [{**skipped, **change}], SHA, RUN_ID)

    def test_duplicate_jobs_and_wrong_native_targets_rejected(self):
        run, jobs = native_evidence()
        with self.assertRaises(ValueError):
            owner.validate_run(run, jobs + [copy.deepcopy(jobs[0])], SHA, RUN_ID)
        bad = copy.deepcopy(jobs)
        for job in bad:
            job["name"] = job["name"].replace("x86_64-apple-darwin", "unknown-target")
        with self.assertRaises(ValueError):
            owner.validate_run(run, bad, SHA, RUN_ID)


class BundleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.bundle = Path(self.temp.name)
        make_bundle(self.bundle)
        self.signature = patch.object(owner, "verify_signature").start()
        self.sboms = patch.object(owner.inventory, "_validate_sboms").start()
        self.addCleanup(patch.stopall)

    def test_complete_bundle_uses_signature_and_shared_inventory_validation(self):
        data, records = owner.validate_bundle(self.bundle)
        self.assertEqual(data["source_commit"], SHA)
        self.assertEqual(len(records), 16)
        self.signature.assert_called_once_with(self.bundle, None)
        self.sboms.assert_called_once()

    def test_signature_failure_stops_validation(self):
        self.signature.side_effect = ValueError("invalid signature")
        with self.assertRaises(ValueError):
            owner.validate_bundle(self.bundle)
        self.sboms.assert_not_called()

    def test_oversized_bundle_is_rejected_before_signature_or_hash_work(self):
        with patch.object(owner.inventory, "MAX_TOTAL_BYTES", 10), self.assertRaises(ValueError):
            owner.validate_bundle(self.bundle)
        self.signature.assert_not_called()

    def test_tampered_artifact_fails_signed_checksum(self):
        next(self.bundle.glob("*.msi")).write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "checksums"):
            owner.validate_bundle(self.bundle)

    def test_extra_missing_and_unknown_packages_are_rejected(self):
        extra = self.bundle / "extra.txt"
        extra.write_text("unexpected")
        with self.assertRaises(ValueError):
            owner.validate_bundle(self.bundle)
        extra.unlink()
        package = next(self.bundle.glob("*.msi"))
        package.rename(self.bundle / "unknown.msi")
        seal(self.bundle)
        with self.assertRaises(ValueError):
            owner.validate_bundle(self.bundle)

    def test_duplicate_and_traversing_checksums_are_rejected(self):
        path = self.bundle / "SHA256SUMS"
        original = path.read_text()
        for bad in (original + original.splitlines()[0] + "\n", original + "a"*64 + "  ../outside\n"):
            path.write_text(bad)
            with self.assertRaises(ValueError):
                owner.validate_bundle(self.bundle)

    def test_misleading_disclosure_or_duplicate_manifest_field_rejected(self):
        path = self.bundle / "release-manifest.json"
        data = json.loads(path.read_text())
        data["os_signing"]["windows"] = "signed"
        path.write_text(json.dumps(data))
        with self.assertRaises(ValueError):
            owner.validate_bundle(self.bundle)
        path.write_text('{"version":"0.0.3-alpha","version":"1.0.0"}')
        with self.assertRaises(ValueError):
            owner.validate_bundle(self.bundle)


class RemoteTests(unittest.TestCase):
    def setUp(self):
        self.records = {"asset.zip": ("a"*64, 20), "SHA256SUMS": ("b"*64, 80)}

    def test_complete_remote_draft_and_published_identity(self):
        self.assertEqual(owner.validate_remote(remote(self.records), self.records, notes="Alpha fixture notes", complete=True), set(self.records))
        owner.validate_remote(remote(self.records, draft=False, immutable=True), self.records, notes="Alpha fixture notes", complete=True, published=True)

    def test_target_commitish_is_informational_and_actual_tag_is_verified_separately(self):
        data = remote(self.records)
        data["target_commitish"] = "main"
        owner.validate_remote(data, self.records, notes="Alpha fixture notes", complete=True)
        with patch.object(owner, "api", side_effect=[{"object": {"type": "tag", "sha": "c"*40}}, {"object": {"type": "commit", "sha": "b"*40}}]), self.assertRaises(ValueError):
            owner.verify_tag(SHA)

    def test_existing_published_release_cannot_be_overwritten(self):
        with self.assertRaises(ValueError):
            owner.validate_remote(remote(self.records, draft=False, immutable=True), self.records, notes="Alpha fixture notes", complete=False)

    def test_exact_partial_draft_can_resume_but_conflicts_cannot(self):
        draft = remote(self.records)
        draft["assets"].pop()
        self.assertEqual(owner.validate_remote(draft, self.records, notes="Alpha fixture notes", complete=False), {"asset.zip"})
        with self.assertRaises(ValueError):
            owner.validate_remote(draft, self.records, notes="Alpha fixture notes", complete=True)
        draft["assets"][0]["digest"] = "sha256:" + "c"*64
        with self.assertRaises(ValueError):
            owner.validate_remote(draft, self.records, notes="Alpha fixture notes", complete=False)

    def test_missing_metadata_or_disabled_immutability_rejected(self):
        for key in ("draft", "prerelease", "target_commitish", "tag_name", "body", "name"):
            data = remote(self.records)
            del data[key]
            with self.assertRaises(ValueError):
                owner.validate_remote(data, self.records, notes="Alpha fixture notes", complete=True)
        with self.assertRaises(ValueError):
            owner.validate_remote(remote(self.records, draft=False), self.records, notes="Alpha fixture notes", complete=True, published=True)

    def test_wrong_owner_disabled_immutability_and_stale_main_fail_closed(self):
        responses = [{"login": owner.REPOSITORY.split("/")[0]}, {"enabled": True}, {"object": {"sha": SHA}}]
        for index, invalid in [(0, {"login": "other"}), (1, {"enabled": False}), (2, {"object": {"sha": "b"*40}})]:
            bad = copy.deepcopy(responses)
            bad[index] = invalid
            with patch.object(owner, "api", side_effect=bad), self.assertRaises(ValueError):
                owner.authorize_owner(SHA, "unsigned-first-alpha")


class PublicationFlowTests(unittest.TestCase):
    def exercise(self, *, perform=True, existing=False, late_disabled=False, upload_failure=False, late_tag_change=False):
        self.writes = []
        self.commands = []
        self.patches = []
        run, jobs = native_evidence()
        records = {"asset.zip": ("a"*64, 20), "SHA256SUMS": ("b"*64, 80)}
        draft = remote(records)
        draft["assets"] = draft["assets"][:1] if existing else []
        immutable_reads = 0
        tag_created = existing
        tag_reads = 0
        notes = b"Alpha fixture notes\n\n## Choose your download\n\nFixture choices.\n\n## Signing and verification\n\nFixture disclosures.\n"
        rendered_notes = owner.replace_download_guide(notes.decode(), owner.alpha.ALPHA_VERSION)
        draft["body"] = rendered_notes

        def fake_api(endpoint, *, fields=None, method="POST", **kwargs):
            nonlocal immutable_reads, tag_created, tag_reads
            if fields is not None:
                self.writes.append((endpoint, method, fields))
            if endpoint == "user":
                return {"login": owner.REPOSITORY.split("/")[0]}
            if endpoint.endswith("immutable-releases"):
                immutable_reads += 1
                return {"enabled": not (late_disabled and immutable_reads > 1)}
            if endpoint.endswith("git/ref/heads/main"):
                return {"object": {"sha": SHA}}
            if "/actions/runs/" in endpoint:
                return {"jobs": jobs, "total_count": len(jobs)} if "/jobs?" in endpoint else run
            if "/contents/" in endpoint:
                return {"encoding": "base64", "size": len(notes), "content": base64.b64encode(notes).decode()}
            if endpoint.endswith("releases?per_page=100"):
                return [copy.deepcopy(draft)] if existing else []
            if "/git/ref/tags/" in endpoint:
                tag_reads += 1
                return {"object": {"type": "tag", "sha": "c"*40}} if tag_created else None
            if "/git/tags/" in endpoint:
                return {"object": {"type": "commit", "sha": "b"*40 if late_tag_change and tag_reads > 1 else SHA}}
            if endpoint.endswith("git/tags"):
                return {"sha": "c"*40}
            if endpoint.endswith("git/refs"):
                tag_created = True
                return {"object": {"sha": "c"*40}}
            if endpoint.endswith("releases"):
                self.assertTrue(fields["draft"])
                self.assertEqual(fields["target_commitish"], SHA)
                self.assertEqual(fields["body"], rendered_notes)
                return copy.deepcopy(draft)
            if endpoint.endswith("releases/45"):
                if method == "PATCH":
                    self.patches.append(fields)
                    draft.update(draft=False, immutable=True)
                return copy.deepcopy(draft)
            raise AssertionError(endpoint)

        def command(argv, **kwargs):
            self.commands.append(argv)
            self.assertNotIn("--clobber", argv)
            if argv[:3] == ["gh", "release", "upload"]:
                if upload_failure:
                    raise ValueError("network stopped upload")
                draft["assets"] = remote(records)["assets"]
            return subprocess.CompletedProcess(argv, 0, b"", b"")

        with patch("builtins.print"), patch.object(owner, "validate_bundle", return_value=(manifest(), records)), \
                patch.object(owner, "api", side_effect=fake_api), \
                patch.object(owner, "checked_process", side_effect=command), \
                patch.object(owner.inventory, "digest", side_effect=lambda path: records[path.name]):
            owner.publish(Path("bundle"), RUN_ID, "unsigned-first-alpha", perform=perform)

    def test_verified_run_publishes_exact_draft_id_then_verifies_attestations(self):
        self.exercise()
        self.assertEqual(len(self.patches), 1)
        self.assertEqual(self.patches[0], {"draft": False, "prerelease": True, "make_latest": "false"})
        self.assertEqual(sum(args[:3] == ["gh", "release", "verify-asset"] for args in self.commands), 2)

    def test_verify_only_never_writes_remote_state(self):
        self.exercise(perform=False)
        self.assertFalse(self.writes)
        self.assertFalse(self.commands)

    def test_matching_partial_draft_uploads_only_missing_assets(self):
        self.exercise(existing=True)
        uploads = [args for args in self.commands if args[:3] == ["gh", "release", "upload"]]
        self.assertEqual(len(uploads), 1)
        self.assertTrue(uploads[0][-1].endswith("SHA256SUMS#Verification - SHA-256 checksums (SHA256SUMS)"))
        self.assertNotIn("bundle/asset.zip", uploads[0])
        self.assertEqual(len(self.writes), 1)  # Only the final PATCH; no new tag/draft.

    def test_late_immutability_change_and_failed_upload_never_publish(self):
        for args in ({"late_disabled": True}, {"upload_failure": True}, {"late_tag_change": True}):
            with self.subTest(args=args), self.assertRaises(ValueError):
                self.exercise(**args)
            self.assertFalse(self.patches)

    def test_administration_denial_is_not_treated_as_missing_release(self):
        denied = subprocess.CompletedProcess([], 1, b'HTTP/2.0 403 Forbidden\ncontent-type: application/json\n\n{"message":"denied"}', b'')
        with patch.object(owner, "checked_process", return_value=denied), self.assertRaisesRegex(ValueError, "403"):
            owner.api("repos/example/terminal/immutable-releases", missing=True)

    def test_signature_verifiers_never_inherit_github_or_signing_credentials(self):
        injected = {key: "unit-fixture" for key in ("GH_TOKEN", "GITHUB_TOKEN", "GH_ENTERPRISE_TOKEN", "GITHUB_ENTERPRISE_TOKEN", "AUTOMEXIA_RELEASE_MINISIGN_SECRET_KEY", "WSLENV")}
        for command in ("wsl", "minisign"):
            with patch.dict(owner.os.environ, injected), patch.object(owner.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, b"", b"")) as run:
                owner.checked_process([command, "--version"])
            self.assertFalse(set(injected) & set(run.call_args.kwargs["env"]))

    def test_boolean_api_fields_are_typed_and_notes_are_literal_argv(self):
        result = subprocess.CompletedProcess([], 0, b'HTTP/2.0 201 Created\ncontent-type: application/json\n\n{"id":45}', b'')
        with patch.object(owner, "checked_process", return_value=result) as command:
            owner.api("repos/example/terminal/releases", fields={"draft": True, "body": "@literal-not-a-file"})
        self.assertEqual(command.call_args.args[0][-4:], ["-F", "draft=true", "-f", "body=@literal-not-a-file"])


if __name__ == "__main__":
    unittest.main()
