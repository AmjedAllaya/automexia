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


class WorkflowBoundaryTests(unittest.TestCase):
    def test_reviewed_workflow_boundary_passes(self) -> None:
        policy.validate_workflow_boundary()

    def test_workflow_protection_mutations_fail_closed(self) -> None:
        cases = [
            ("automexia-command-productivity/src/actions/workflow.rs", "if status != 0", "if false"),
            ("automexia-command-productivity/src/actions/workflow.rs", "observed.input_revision != self.baseline.input_revision", "false"),
            ("automexia-command-productivity/src/actions/workflow.rs", "MAX_WORKFLOW_STEPS: usize = 32", "MAX_WORKFLOW_STEPS: usize = 320"),
            ("apps/automexia-terminal/src/screen/action_surface/workflow.rs", "if !unchanged", "if false"),
            ("apps/automexia-terminal/src/screen/action_surface/workflow.rs", "self.invalidate_workflow_submission();", "/* no cancellation */"),
            ("rio-vt/src/crosswords/command_actions.rs", "self.workflow_input.submitted == Some(receipt)", "false"),
            ("rio-vt/src/performer/mod.rs", "terminal.accept_workflow_submission(receipt)", "true"),
            ("rio-vt/src/event/mod.rs", "reviewed_command: false", "reviewed_command: true"),
            (policy.QUICK_ACTION_WORKER, "pending.mutations.len() >= 8", "false"),
            (policy.QUICK_ACTION_WORKER, "let mutations = std::mem::take(&mut lock(&self.pending.0).mutations);", "let mutations = ();"),
        ]
        original = policy.bounded_text
        for relative, before, after in cases:
            target = policy.ROOT / relative
            self.assertIn(before, original(target))
            def mutated(path, *args, **kwargs):
                source = original(path, *args, **kwargs)
                return source.replace(before, after) if path == target else source
            with self.subTest(relative=relative, before=before), mock.patch.object(policy, "bounded_text", side_effect=mutated):
                with self.assertRaises(policy.Cp22Error):
                    policy.validate_workflow_boundary()


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


class Cp22LifecycleMutationTests(unittest.TestCase):
    """Mutate real owners, retaining inert evidence to expose disconnected paths."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.contract = json.loads(policy.bounded_text(policy.CONTRACT))
        cls.worker = "apps/automexia-terminal/src/automexia/quick_actions/worker.rs"
        cls.extension = "automexia-extension-runtime/src/lib.rs"
        cls.router = "apps/automexia-terminal/src/router/mod.rs"
        cls.sources = {
            relative: policy.bounded_text(policy.ROOT / relative)
            for relative in (cls.worker, cls.extension, cls.router)
        }

    def reject_mutation(self, relative, original, replacement, decoy="") -> None:
        source = self.sources[relative]
        self.assertEqual(source.count(original), 1, "mutation must target one real owner")
        changed = source.replace(original, replacement, 1) + decoy
        original_read = policy.bounded_text

        def mutated(path, maximum=policy.MAX_POLICY_BYTES):
            key = path.relative_to(policy.ROOT).as_posix()
            result = changed if key == relative else original_read(path, maximum)
            if key == self.worker:
                # The inherited checker demanded this obsolete application token.
                # Supply it only as an inert comment, so RED demonstrates that the
                # old gate actually admits a disconnected production lifecycle.
                result += "\n// Inert old-owner proof: handle.join()\n"
            return result

        with mock.patch.object(policy, "bounded_text", side_effect=mutated):
            with self.assertRaisesRegex(policy.Cp22Error, "Quick Action (?:lifecycle|Drop)"):
                policy.validate_sources(self.contract)

    def test_application_cannot_discard_the_admitted_worker(self) -> None:
        self.reject_mutation(self.worker, "worker: Some(worker),", "worker: None,")

    def test_kickoff_capacity_cannot_expand(self) -> None:
        self.reject_mutation(
            self.worker,
            'BoundedWorker::new("automexia-quick-actions", 1, run_worker)',
            'BoundedWorker::new("automexia-quick-actions", 2, run_worker)',
        )

    def test_kickoff_cannot_lose_its_real_handler(self) -> None:
        self.reject_mutation(
            self.worker,
            'BoundedWorker::new("automexia-quick-actions", 1, run_worker)',
            'BoundedWorker::new("automexia-quick-actions", 1, |_run| {})',
        )

    def test_pending_cleanup_cannot_bind_a_different_queue(self) -> None:
        self.reject_mutation(
            self.worker,
            "let pending_cleanup = PendingCleanup {\n        pending: Arc::clone(&shared.pending),\n    };",
            "let pending_cleanup = PendingCleanup {\n        pending: Arc::new((Mutex::new(PendingState::default()), Condvar::new())),\n    };",
        )

    def test_queued_cleanup_guard_cannot_drop_at_loop_entry(self) -> None:
        self.reject_mutation(
            self.worker,
            "        _pending_cleanup,\n    } = run;",
            "        _pending_cleanup: _,\n    } = run;",
        )

    def test_queued_cleanup_guard_cannot_be_explicitly_dropped_early(self) -> None:
        self.reject_mutation(
            self.worker,
            "        _pending_cleanup,\n    } = run;",
            "        _pending_cleanup,\n    } = run;\n    drop(_pending_cleanup);",
        )

    def test_cancellation_cannot_drain_foreign_callbacks_on_the_caller(self) -> None:
        self.reject_mutation(
            self.worker,
            "            state.shutdown = true;",
            "            state.shutdown = true;\n            state.latest_by_route.clear();",
        )

    def test_application_cancellation_cannot_disconnect_worker_retirement(self) -> None:
        self.reject_mutation(
            self.worker,
            "            worker.request_shutdown();",
            "            let _ = worker.shutdown_status();",
            "\n/* worker.request_shutdown(); */\n",
        )

    def test_drop_cannot_restore_synchronous_native_cleanup(self) -> None:
        self.reject_mutation(
            self.worker,
            "impl Drop for RuntimeInner {\n    fn drop(&mut self) {\n        self.request_shutdown();\n    }\n}",
            "impl Drop for RuntimeInner {\n    fn drop(&mut self) {\n        self.request_shutdown();\n        if let Some(worker) = &self.worker { worker.shutdown_timeout(Duration::from_secs(1)); }\n    }\n}",
        )

    def test_timeout_cannot_claim_success_before_the_owned_acknowledgement(self) -> None:
        self.reject_mutation(
            self.worker,
            "        let started = Instant::now();\n        self.request_shutdown();",
            "        return true;\n        let started = Instant::now();\n        self.request_shutdown();",
        )

    def test_native_worker_cannot_detach_from_the_cleanup_owner(self) -> None:
        self.reject_mutation(
            self.extension,
            "        cleanup.own(job);",
            "        drop(job);",
            "\nfn cp22_dead_proof(cleanup: &CleanupService, job: JoinJob) { cleanup.own(job); }\n"
            'const CP22_QUOTED_PROOF: &str = r###"/* { */ cleanup.own(job); }"###;\n'
            "/* outer /* nested */ cleanup.own(job); */\n",
        )

    def test_cleanup_mailbox_cannot_discard_the_native_job(self) -> None:
        self.reject_mutation(
            self.extension,
            "        *pending = Some(job);",
            "        drop(job);",
            "\n/* *pending = Some(job); */\n",
        )

    def test_native_handle_cannot_use_an_unrelated_completion(self) -> None:
        self.reject_mutation(
            self.extension,
            "            completion: Arc::clone(&completion),",
            "            completion: Arc::new(JoinCompletion::default()),",
        )

    def test_cleanup_thread_must_retain_the_admission_permit(self) -> None:
        self.reject_mutation(
            self.extension,
            "                let _permit = permit;",
            "                drop(permit);",
        )

    def test_actual_join_cannot_be_replaced_with_a_success_claim(self) -> None:
        self.reject_mutation(
            self.extension,
            "                    match job.handle.join() {",
            "                    match Ok::<(), Box<dyn std::any::Any + Send>>(()) {",
            "\n// match job.handle.join() { Ok(()) => job.completion.finish(), }\n",
        )

    def test_cleanup_acknowledgement_cannot_precede_the_actual_join(self) -> None:
        self.reject_mutation(
            self.extension,
            "                    match job.handle.join() {",
            "                    job.completion.finish();\n                    match job.handle.join() {",
        )

    def test_router_shutdown_cannot_disconnect_quick_action_cancellation(self) -> None:
        self.reject_mutation(
            self.router,
            "        self.quick_actions.request_shutdown();",
            "        let _ = self.quick_actions.shutdown_status();",
            "\n// self.quick_actions.request_shutdown();\n",
        )


if __name__ == "__main__":
    unittest.main()
