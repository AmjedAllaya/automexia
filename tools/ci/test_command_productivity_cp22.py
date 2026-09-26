#!/usr/bin/env python3
"""Mutation tests for the active CP2.2 Quick Action contract."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock


sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_command_productivity_cp22 as policy  # noqa: E402


class Cp22ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))

    def validate_mutation(self, mutate) -> None:
        document = copy.deepcopy(self.contract)
        mutate(document)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaises(policy.Cp22Error):
                policy.load_contract(path)

    def test_canonical_repository_contract_and_sources_pass(self) -> None:
        counts = policy.validate_repository()
        self.assertEqual(counts["model_files"], 5)
        self.assertEqual(counts["tests"], 13)

    def test_resource_ceiling_expansion_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["limits"].__setitem__("query_bytes", 1_000_000)
        )

    def test_stale_or_competing_model_owner_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["ownership"].__setitem__(
                "model", "automexia-devops-capability-free"
            )
        )

    def test_network_or_exact_launch_authority_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["capabilities"].__setitem__("network", True)
        )
        self.validate_mutation(
            lambda document: document["capabilities"].__setitem__("exact_launch", True)
        )

    def test_workspace_or_secret_activation_is_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document.__setitem__("workspace_activation", "automatic")
        )
        self.validate_mutation(
            lambda document: document.__setitem__("secret_expansion", "plaintext")
        )

    def test_precedence_changes_and_duplicate_sources_are_rejected(self) -> None:
        self.validate_mutation(
            lambda document: document["scope_precedence"].reverse()
        )
        self.validate_mutation(
            lambda document: document["model_files"].append(document["model_files"][0])
        )

    def test_linked_policy_input_is_rejected_where_supported(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target.json"
            linked = root / "linked.json"
            target.write_text("{}", encoding="utf-8")
            try:
                linked.symlink_to(target)
            except OSError:
                self.skipTest("symbolic links are unavailable")
            with self.assertRaises(policy.Cp22Error):
                policy.bounded_text(linked)

    def test_collapsed_user_scope_or_global_pending_slot_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "activation.rs":
                source = source.replace("LayerIdentity::ShellUser", "LayerIdentity::User")
            if path.name == "worker.rs":
                source = source.replace("latest_by_route", "latest")
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaises(policy.Cp22Error):
                policy.validate_sources(self.contract)

    def test_secret_prompt_preflight_or_visible_health_removal_is_rejected(self) -> None:
        original = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            source = original(path, maximum)
            if path.name == "action_surface.rs":
                source = source.replace(
                    "unavailable_before_placeholder", "post_prompt_unavailable"
                )
            return source

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaises(policy.Cp22Error):
                policy.validate_sources(self.contract)


class Cp22WorkerOwnershipTests(unittest.TestCase):
    """Supplemental real-source mutations; native Rust tests own behavior."""

    @classmethod
    def setUpClass(cls):
        cls.worker = policy.bounded_text(policy.ROOT / policy.WORKER_SOURCE)
        cls.runtime = policy.bounded_text(policy.ROOT / policy.WORKER_RUNTIME_SOURCE)
        # Validate the positive control before any mutation. Otherwise a bad
        # baseline makes unrelated negative cases pass for the wrong reason.
        policy._validate_worker_lifecycle_sources(cls.worker, cls.runtime)

    def assert_rejected(self, owner, old, new):
        source = self.worker if owner == "worker" else self.runtime
        self.assertIn(old, source, "mutation must alter the real source")
        changed = source.replace(old, new, 1)
        with self.assertRaises(policy.Cp22Error):
            policy._validate_worker_lifecycle_sources(
                changed if owner == "worker" else self.worker,
                changed if owner == "runtime" else self.runtime,
            )

    def test_cp22_real_shared_owner_is_accepted(self):
        policy._validate_worker_lifecycle_sources(self.worker, self.runtime)
        self.assertNotIn(".join()", policy._cp22_worker_code(self.worker))
        self.assertIn("job.handle.join()", policy._cp22_worker_code(self.runtime))

    def test_cp22_crlf_and_call_whitespace_are_accepted(self):
        worker = self.worker.replace("\r\n", "\n").replace("\n", "\r\n")
        runtime = self.runtime.replace("job.handle.join()", "job.handle . join ( )")
        policy._validate_worker_lifecycle_sources(worker, runtime)

    def test_cp22_worker_owner_and_admission_are_required(self):
        for old, new in (("Option<BoundedWorker<WorkerRun>>", "Option<DetachedWorker<WorkerRun>>"),
                         ('BoundedWorker::new("automexia-quick-actions", 1, run_worker)',
                          'BoundedWorker::new("automexia-quick-actions", 2, run_worker)'),
                         ("worker.try_submit(run)", "pretend_to_submit(run)")):
            with self.subTest(old=old): self.assert_rejected("worker", old, new)

    def test_cp22_shutdown_must_revoke_publication(self):
        for old in ("state.shutdown = true;", "lock(&self.latest_requested).clear();",
                    "lock(&self.results).clear();", "lock(&self.workspace_authorizations).clear();",
                    "lock(&self.provider_snapshots).clear();", "self.pending.1.notify_all();"):
            with self.subTest(old=old): self.assert_rejected("worker", old, "")

    def test_cp22_shutdown_delegation_cannot_be_disconnected(self):
        self.assert_rejected("worker", "worker.request_shutdown();", "")
        self.assert_rejected("worker", "self.0.request_shutdown();", "")

    def test_cp22_caller_budget_and_boolean_result_are_preserved(self):
        old = "worker.shutdown_timeout(timeout.saturating_sub(started.elapsed()))"
        for new in ("true", "worker.shutdown_timeout(timeout)", old + "; true"):
            with self.subTest(new=new): self.assert_rejected("worker", old, new)

    def test_cp22_drop_cannot_block_or_omit_retirement(self):
        old = "impl Drop for RuntimeInner {\n    fn drop(&mut self) {\n        self.request_shutdown();"
        source = self.worker.replace("\r\n", "\n")
        self.assertIn(old, source)
        for ending in ("", "handle.join();", "self.request_shutdown(); handle.join();"):
            mutation = source.replace(old, old.rsplit("self.request_shutdown();", 1)[0] + ending, 1)
            with self.subTest(ending=ending), self.assertRaises(policy.Cp22Error):
                policy._validate_worker_lifecycle_sources(mutation, self.runtime)

    def test_cp22_full_queue_shutdown_cannot_use_blocking_send(self):
        self.assert_rejected("runtime", "self.sender.try_send(WorkerMessage::Shutdown)",
                             "self.sender.send(WorkerMessage::Shutdown)")

    def test_cp22_shared_join_is_required(self):
        for new in ("pretend_join()", "job.handle.is_finished()", "Ok(())"):
            with self.subTest(new=new):
                self.assert_rejected("runtime", "job.handle.join()", new)

    def test_cp22_join_failure_must_not_become_success(self):
        self.assert_rejected("runtime", "job.completion.fail();", "job.completion.finish();")
        self.assert_rejected("runtime", "Ok(()) => job.completion.finish(),", "Ok(()) => (),")

    def test_cp22_early_acknowledgement_is_rejected(self):
        self.assert_rejected("runtime", "match job.handle.join() {",
                             "job.completion.finish(); match job.handle.join() {")

    def test_cp22_strings_and_comments_cannot_supply_join_evidence(self):
        for new in ('match Ok(()) { /* job.handle.join() */',
                    'let fake = "match job.handle.join() {"; match Ok(()) {'):
            with self.subTest(new=new):
                self.assert_rejected("runtime", "match job.handle.join() {", new)

    def test_cp22_registrations_and_failure_gate_are_required(self):
        self.assert_rejected("runtime", "self.registrations.load(Ordering::Acquire) == 0", "true")
        self.assert_rejected("runtime", "!self.failed.load(Ordering::Acquire)", "true")

    def test_cp22_shared_wait_cannot_fabricate_completion(self):
        self.assert_rejected("runtime", "completion.wait(timeout)", "true")

    def test_cp22_join_owner_must_precede_worker_publication(self):
        self.assert_rejected("runtime", "cleanup.own(job);", "")
        self.assert_rejected("runtime", "worker.completion.complete()", "true")
        source = self.runtime.replace("\r\n", "\n")
        old = "cleanup.own(job);\n        slot.worker = Some(worker);"
        self.assertIn(old, source)
        changed = source.replace(old, "slot.worker = Some(worker);\n        cleanup.own(job);", 1)
        with self.assertRaises(policy.Cp22Error):
            policy._validate_worker_lifecycle_sources(self.worker, changed)

    def test_cp22_unrelated_helper_cannot_replace_shared_join(self):
        runtime = self.runtime.replace("job.handle.join()", "pretend_join()", 1)
        runtime = "fn unrelated() { match job.handle.join() { Ok(()) => job.completion.finish(), Err(_) => () } }\n" + runtime
        with self.assertRaises(policy.Cp22Error):
            policy._validate_worker_lifecycle_sources(self.worker, runtime)

    def test_cp22_canonical_source_validator_dispatches_owner_check(self):
        with mock.patch.object(policy, "validate_worker_lifecycle", side_effect=policy.Cp22Error("owner-dispatch-canary")):
            with self.assertRaisesRegex(policy.Cp22Error, "owner-dispatch-canary"):
                policy.validate_sources({"model_files": []})


    def test_cp22_path_join_arguments_are_accepted(self):
        # The production startup path uses both of the first two calls. The
        # remaining forms ensure empty, computed and whitespace-bearing path
        # arguments are not confused with zero-argument handle joins.
        for call in (
            'rio_backend::config::config_dir_path().join("actions")',
            'store.root().join("workspace-trust")',
            'root.join("")',
            'root.join(WORKSPACE_ACTION_DIRECTORY_NAME)',
            'root.join(Path::new("relative"))',
            'root . join ( /* path, not thread */ "actions", )',
        ):
            worker = "fn cp22_path_canary() { let _ = " + call + "; }\n" + self.worker
            with self.subTest(call=call):
                policy._validate_worker_lifecycle_sources(worker, self.runtime)

    def test_cp22_string_slice_joins_are_accepted(self):
        for call in ('parts.join(" ")', 'parts.join("")', 'parts.join(separator)'):
            worker = "fn cp22_string_canary() { let _ = " + call + "; }\n" + self.worker
            with self.subTest(call=call):
                policy._validate_worker_lifecycle_sources(worker, self.runtime)

    def test_cp22_zero_argument_thread_joins_are_rejected(self):
        for call in ("handle.join()", "handle . join ( )",
                     "handle.join(/* native wait */)", "self.handle.take()?.join()"):
            worker = "fn cp22_wait_canary() { let _ = " + call + "; }\n" + self.worker
            with self.subTest(call=call), self.assertRaisesRegex(
                    policy.Cp22Error, "native thread join must remain"):
                policy._validate_worker_lifecycle_sources(worker, self.runtime)

    def test_cp22_associated_native_join_calls_are_rejected(self):
        for call in ("JoinHandle::join(handle)",
                     "std::thread::JoinHandle::<()>::join(handle)",
                     "ScopedJoinHandle::join(handle)",
                     "std::thread::JoinHandle::join"):
            worker = "fn cp22_wait_canary() { let _ = " + call + "; }\n" + self.worker
            with self.subTest(call=call), self.assertRaisesRegex(
                    policy.Cp22Error, "native thread join must remain"):
                policy._validate_worker_lifecycle_sources(worker, self.runtime)

    def test_cp22_test_only_thread_joins_are_not_production_owners(self):
        production = self.worker.replace("\r\n", "\n").split("\n#[cfg(test)]\nmod tests", 1)[0]
        worker = production + "\n#[cfg(test)]\nmod tests { fn fixture() { handle.join(); } }\n"
        policy._validate_worker_lifecycle_sources(worker, self.runtime)

    def test_cp22_path_join_acceptance_does_not_hide_lifecycle_removal(self):
        worker = 'fn cp22_path_canary() { root.join("actions"); }\n' + self.worker
        policy._validate_worker_lifecycle_sources(worker, self.runtime)
        for owner, old, new, message in (
            ("worker", "worker.request_shutdown();", "", "cancel publication"),
            ("runtime", "job.handle.join()", "pretend_join()", "cleanup must acknowledge"),
            ("runtime", "cleanup.own(job);", "", "cleanup service must own"),
        ):
            source = worker if owner == "worker" else self.runtime
            self.assertIn(old, source)
            changed = source.replace(old, new, 1)
            with self.subTest(owner=owner, token=old), self.assertRaisesRegex(policy.Cp22Error, message):
                policy._validate_worker_lifecycle_sources(
                    changed if owner == "worker" else worker,
                    changed if owner == "runtime" else self.runtime)


if __name__ == "__main__":
    unittest.main()
