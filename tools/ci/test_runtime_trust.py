#!/usr/bin/env python3
"""Mutation tests for runtime and release trust boundaries."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import unittest

import yaml


MODULE_PATH = Path(__file__).with_name("check_runtime_trust.py")
SPEC = importlib.util.spec_from_file_location("automexia_runtime_trust", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_runtime_trust.py")
TRUST = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TRUST)
ROOT = MODULE_PATH.parents[2]


class RuntimeTrustTests(unittest.TestCase):
    def read(self, path: str) -> str:
        # Source mutations deliberately traverse the checker-facing repository
        # text; they are not substitutes for runtime tests of the owning adapters.
        return (ROOT / path).read_text(encoding="utf-8")

    def test_welcome_input_defers_configuration_file_io(self) -> None:
        source = self.read("apps/automexia-terminal/src/router/mod.rs")
        input_handler = TRUST.section(source, "pub fn has_key_wait(", "pub struct Router")
        self.assertFalse(
            "create_config_file(" in input_handler,
            "Welcome key handling must publish an intent instead of writing configuration",
        )

    def test_welcome_worker_ownership_mutations_are_rejected(self) -> None:
        router = self.read("apps/automexia-terminal/src/router/mod.rs")
        application = self.read("apps/automexia-terminal/src/application.rs")
        worker = self.read("apps/automexia-terminal/src/config_creation.rs")
        TRUST.validate_welcome_input(router, application, worker)
        mutations = [
            (router.replace("return welcome_key_intent(", "create_config_file(None); return welcome_key_intent(", 1), application, worker),
            (router.replace("RouteKeyIntent::CreateConfiguration", "RouteKeyIntent::Consumed", 1), application, worker),
            (router, application.replace("let intent = route.has_key_wait(", "create_config_file(None); let intent = route.has_key_wait(", 1), worker),
            (router, application.replace("self.router.config_creation.submit(", "other_worker.submit(", 1), worker),
            (router, application.replace("self.finish_configuration_creation();", "", 1), worker),
            (router, application.replace("completion.applies_to(", "always_publish(", 1), worker),
            (router, application, worker.replace("mpsc::sync_channel(1)", "mpsc::channel()", 1)),
            (router, application, worker.replace('BoundedWorker::new("starter-config", 1,', 'BoundedWorker::new("starter-config", 2,', 1)),
            (router, application, worker.replace("completed.try_send(completion).is_ok()", "true", 1)),
            (router, application, worker.replace("wake.wake(window);", "", 1)),
        ]
        for index, sources in enumerate(mutations):
            with self.subTest(mutation=index), self.assertRaises(TRUST.RuntimeTrustError):
                TRUST.validate_welcome_input(*sources)

    def test_windows_event_loop_registers_wake_before_creating_target(self) -> None:
        source = self.read("rio-window/src/platform_impl/windows/event_loop.rs")
        constructor = TRUST.section(source, "pub(crate) fn new(", "pub fn window_target(")
        self.assertTrue(
            "create_event_target_after_registration(" in constructor,
            "Windows wake registration must fail before creating a live event target",
        )

    def test_windows_wake_registration_order_mutations_are_rejected(self) -> None:
        source = self.read("rio-window/src/platform_impl/windows/event_loop.rs")
        TRUST.validate_windows_wake_registration(source)
        statement = "register().map_err(|error| EventLoopError::Os(os_error!(error)))?;"
        mutations = [
            source.replace("USER_EVENT_MSG_ID.try_get()", "OTHER_MSG_ID.try_get()", 1),
            source.replace("let thread_msg_target = create_event_target_after_registration(", "let thread_msg_target = unchecked_target(", 1),
            source.replace(statement, "", 1),
            source.replace(statement, "Ok(create_target()); " + statement, 1),
            source.replace(statement, statement.replace("?;", ";"), 1),
        ]
        for index, altered in enumerate(mutations):
            with self.subTest(mutation=index), self.assertRaises(TRUST.RuntimeTrustError):
                TRUST.validate_windows_wake_registration(altered)

    def test_current_repository_satisfies_the_contract(self) -> None:
        TRUST.validate_repository()

    def test_automatic_shell_installer_is_rejected(self) -> None:
        source = self.read("tools/xtask/src/main.rs")
        altered = source.replace(
            "LaunchPhase::Smoke,",
            "LaunchPhase::Smoke, LaunchPhase::InstallShellIntegration,",
            1,
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "must not provision"):
            TRUST.validate_launcher(altered)

    def test_maintainer_execution_policy_bypass_is_rejected(self) -> None:
        source = self.read("tools/xtask/src/main.rs")
        altered = source.replace(
            '"-NoProfile",',
            '"-NoProfile", "-ExecutionPolicy", "Bypass",',
            1,
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "execution policy"):
            TRUST.validate_launcher(altered)

    def test_qa_execution_policy_bypass_is_rejected(self) -> None:
        source = self.read("tools/ci/qa.py").replace(
            '"-NonInteractive",',
            '"-ExecutionPolicy", "Bypass",',
            1,
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "QA PowerShell"):
            TRUST.validate_qa_runner(source)

    def test_environment_validation_and_trust_order_cannot_disappear_or_move(self) -> None:
        source = self.read("apps/automexia-terminal/src/main.rs")
        validation = "rio_backend::config::environment::parse_environment(&config.env_vars)?;"
        assignment = "std::env::set_var(name, value);"
        trust = "automexia::shell_integration::prepare_session_environment();"
        first_write = 'std::env::set_var("TERM_PROGRAM", product::TERM_PROGRAM);'
        self.assertEqual(source.count(validation), 1)
        self.assertEqual(source.count(assignment), 1)
        self.assertEqual(source.count(trust), 1)
        TRUST.validate_application(
            source,
            self.read("apps/automexia-terminal/src/automexia/shell.rs"),
            self.read("apps/automexia-terminal/src/automexia/shell_integration.rs"),
            self.read("shell-integration/cmd/automexia-ls.cmd"),
        )
        mutations = {
            "missing validation": source.replace(validation, "Vec::new();", 1),
            "validation after first write": source.replace(validation, "Vec::new();", 1).replace(first_write, first_write + validation, 1),
            "missing application loop": source.replace("for (name, value) in environment {", "for (name, value) in other_batch {", 1),
            "missing trust resolution": source.replace(trust, "", 1),
            "trust before configured values": source.replace(trust, "", 1).replace(assignment, trust + assignment, 1),
        }
        for name, altered in mutations.items():
            with self.subTest(mutation=name), self.assertRaisesRegex(
                TRUST.RuntimeTrustError, "environment validation|session integration"
            ):
                TRUST.validate_application(
                    altered,
                    self.read("apps/automexia-terminal/src/automexia/shell.rs"),
                    self.read("apps/automexia-terminal/src/automexia/shell_integration.rs"),
                    self.read("shell-integration/cmd/automexia-ls.cmd"),
                )

    def test_verbatim_windows_integration_root_is_rejected(self) -> None:
        integration = self.read(
            "apps/automexia-terminal/src/automexia/shell_integration.rs"
        ).replace("dunce::canonicalize(path)", "path.canonicalize()", 1)
        with self.assertRaisesRegex(
            TRUST.RuntimeTrustError, "PowerShell-compatible canonical path"
        ):
            TRUST.validate_application(
                self.read("apps/automexia-terminal/src/main.rs"),
                self.read("apps/automexia-terminal/src/automexia/shell.rs"),
                integration,
                self.read("shell-integration/cmd/automexia.cmd"),
            )

    def test_wildcard_powershell_bootstrap_path_is_rejected(self) -> None:
        shell = self.read(
            "apps/automexia-terminal/src/automexia/shell.rs"
        ).replace("Test-Path -LiteralPath $p", "Test-Path $p", 1)
        with self.assertRaisesRegex(
            TRUST.RuntimeTrustError, "literal paths, preserve policy"
        ):
            TRUST.validate_application(
                self.read("apps/automexia-terminal/src/main.rs"),
                shell,
                self.read(
                    "apps/automexia-terminal/src/automexia/shell_integration.rs"
                ),
                self.read("shell-integration/cmd/automexia.cmd"),
            )

    def test_uncaught_powershell_policy_failure_is_rejected(self) -> None:
        shell = self.read(
            "apps/automexia-terminal/src/automexia/shell.rs"
        ).replace(
            "catch [System.Management.Automation.PSSecurityException]",
            "catch [System.IO.IOException]",
            1,
        )
        with self.assertRaisesRegex(
            TRUST.RuntimeTrustError, "literal paths, preserve policy"
        ):
            TRUST.validate_application(
                self.read("apps/automexia-terminal/src/main.rs"),
                shell,
                self.read(
                    "apps/automexia-terminal/src/automexia/shell_integration.rs"
                ),
                self.read("shell-integration/cmd/automexia.cmd"),
            )

    def test_elevated_windows_manifest_is_rejected(self) -> None:
        manifest = self.read("packaging/windows/automexia.manifest").replace(
            'level="asInvoker"', 'level="requireAdministrator"'
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "asInvoker"):
            TRUST.validate_manifest(
                manifest, self.read("apps/automexia-terminal/build.rs")
            )

    def test_encoded_wsl_shell_transport_is_rejected(self) -> None:
        source = self.read("shell-integration/windows-wsl.ps1").replace(
            "--exec sh -s", '--exec sh -c "base64 -d | sh"'
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "bounded raw|encoded or nested"):
            TRUST.validate_wsl(source)

    def test_unsigned_script_packaging_is_rejected(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        altered["jobs"]["sign-windows-runtime"]["steps"] = [
            step
            for step in altered["jobs"]["sign-windows-runtime"]["steps"]
            if step.get("name") != "Sign runtime inputs with Azure Artifact Signing"
        ]
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "recursively sign"):
            TRUST.validate_release(altered)

    def test_packaging_cannot_bypass_isolated_signed_runtime_inputs(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        altered["jobs"]["package-windows-unsigned"]["needs"].remove(
            "sign-windows-runtime"
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "revalidate"):
            TRUST.validate_release(altered)

    def test_publication_cannot_bypass_cold_build_comparison(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        altered["jobs"]["publish"]["needs"].remove("reproducibility-linux")
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "cold-build reproducibility"):
            TRUST.validate_release(altered)

    def test_cold_build_evidence_must_use_the_canonical_script(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        for step in altered["jobs"]["reproducibility-linux"]["steps"]:
            if step.get("name") == "Verify two independent cold source builds":
                step["run"] = "echo skipped"
                break
        else:
            self.fail("canonical cold-build step missing from release workflow fixture")
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "cold-build reproducibility"):
            TRUST.validate_release(altered)

    def test_mutable_or_overwriting_publication_is_rejected(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        publish_release = altered["jobs"]["publish-release"]
        for step in publish_release["steps"]:
            if step.get("name") == "Create immutable version tag and publish verified release":
                step["run"] = str(step["run"]).replace(
                    'gh release upload "$tag"',
                    'gh release upload "$tag" --clobber',
                )
                break
        else:
            self.fail("final publication step missing from release workflow fixture")
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "asset overwrites"):
            TRUST.validate_release(altered)

    def test_package_only_sbom_input_is_rejected(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        publish = altered["jobs"]["publish"]
        for step in publish["steps"]:
            if "anchore/sbom-action@" in str(step.get("uses", "")):
                step["with"]["path"] = "packages"
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "locked Rust dependency|enriched"):
            TRUST.validate_release(altered)

    def test_unbounded_portable_resources_are_rejected(self) -> None:
        xtask = self.read("tools/xtask/src/main.rs").replace(
            "copy_bounded_resource_tree(\n        &root().join",
            "copy_resource_tree(\n        &root().join",
            1,
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "bounded"):
            TRUST.validate_packaging(
                xtask, self.read("packaging/windows/automexia-arm64.wxs")
            )


if __name__ == "__main__":
    unittest.main()
