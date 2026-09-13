"""Mutations of actual source boundaries; native behavior is a separate test."""
import unittest

import check_prompt_discovery as policy


class PromptDiscoveryTests(unittest.TestCase):
    def test_current_boundaries(self):
        policy.validate(policy.sources())

    def test_loss_of_limits_cancellation_ownership_and_caller_is_rejected(self):
        for owner, before, after in [
            ("helper", "std::env::current_exe()", 'Some("unreviewed")'),
            ("helper", "cancellation.is_cancelled()", "false"),
            ("helper", "cancellation.is_cancelled()", "/* cancellation.is_cancelled() */ false"),
            ("helper", "start.elapsed() >= deadline", "false"),
            ("helper", "stop_and_reap(&mut child)", "drop(child)"),
            ("helper", "child.try_wait()", "None"),
            ("helper", "PeekNamedPipe", "blocking_read"),
            ("helper", "const MAX_REQUEST: usize = 8192;", "const MAX_REQUEST: usize = 81920;"),
            ("helper", "const MAX_RESPONSE: usize = 4096;", "const MAX_RESPONSE: usize = 40960;"),
            ("helper", "Duration::from_millis(1500)", "Duration::from_millis(15000)"),
            ("helper", "Duration::from_millis(10)", "Duration::from_millis(100)"),
            ("runtime", "super::prompt_discovery::refresh(", "missing_refresh("),
            ("runtime", "session.environment.hash(&mut hasher)", "session.title.hash(&mut hasher)"),
            ("context", "let kubernetes = if view.wsl.is_some()", "let kubernetes = if false"),
            ("parser", "max_depth: 32", "max_depth: 320"),
            ("parser", "max_depth: 32,", "/* max_depth: 32, */ max_depth: 320,"),
            ("parser", "max_events: 20_000", "max_events: 200_000"),
            ("parser", "max_documents: 1", "max_documents: 10"),
            ("parser", "const MAX_CONTEXTS: usize = 256;", "const MAX_CONTEXTS: usize = 2560;"),
        ]:
            with self.subTest(owner=owner, mutation=before):
                sources = policy.sources()
                self.assertIn(before, sources[owner])
                sources[owner] = sources[owner].replace(before, after)
                with self.assertRaises(ValueError):
                    policy.validate(sources)

    def test_provider_launch_logging_and_host_fallback_are_rejected(self):
        for owner, injection in [
            ("helper", 'Command::new("kubectl");'),
            ("helper", 'tracing::debug!("request");'),
            ("parser", "std::process::Command::new(program);"),
            ("context", "kubernetes_context(host_home, true);"),
        ]:
            sources = policy.sources()
            sources[owner] = injection + sources[owner]
            with self.assertRaises(ValueError):
                policy.validate(sources)

    def test_helper_must_dispatch_before_startup(self):
        sources = policy.sources()
        sources["main"] = sources["main"].replace("prompt_discovery::dispatch_helper()", "removed_dispatch()") + "\nprompt_discovery::dispatch_helper()"
        with self.assertRaises(ValueError):
            policy.validate(sources)

    def test_latency_and_progress_guards_cannot_be_removed(self):
        for owner, before, after in [
            ("runtime", "same_devops_context(&capsule.session, session)", "capsule.session == *session"),
            ("runtime", "self.devops_snapshot(session_id).is_some()", "false"),
            ("runtime", "publish_devops_progress(&request, &snapshot);", ""),
            ("renderer", "self.request_in_flight = self.refresh_pending", "self.request_in_flight = false"),
            ("renderer", "runtime::same_devops_context(previous, session)", "previous == session"),
        ]:
            with self.subTest(owner=owner, mutation=before):
                sources = policy.sources()
                self.assertIn(before, sources[owner])
                sources[owner] = sources[owner].replace(before, after)
                with self.assertRaises(ValueError):
                    policy.validate(sources)


if __name__ == "__main__":
    unittest.main()
