#!/usr/bin/env python3
"""Validate the semantic M8-M12 multi-cloud stable-release contract."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/connection-hub/m8-m12-multicloud-contract-v1.json"
MAX_EVIDENCE_BYTES = 1_048_576
EXPECTED_PROVIDERS = ["aws", "azure", "gcp", "kubernetes", "openshift", "teleport"]
EXPECTED_LIMITS = {
    "aws_config_bytes": 1_048_576,
    "aws_profiles": 128,
    "azure_account_bytes": 262_144,
    "azure_accounts": 128,
    "azure_json_nodes": 4_096,
    "azure_json_depth": 32,
    "gcp_config_bytes": 262_144,
    "gcp_sections": 64,
    "gcp_entries": 512,
    "kubeconfig_bytes": 1_048_576,
    "kubeconfig_sources": 16,
    "kubeconfig_items": 256,
    "teleport_status_bytes": 262_144,
    "teleport_profiles": 32,
    "provider_transients": 16,
    "provider_transient_age_seconds": 86_400,
    "product_providers": 6,
}
EXPECTED_AUTHORITIES = {
    "default_enabled": False,
    "execution_enabled": False,
    "adapter_process_owner": False,
    "adapter_network_owner": False,
    "adapter_filesystem_owner": True,
    "adapter_credential_owner": False,
    "adapter_pty_owner": False,
    "shell_evaluation": False,
    "implicit_enter": False,
    "reviewed_process_capability": True,
    "reviewed_network_capability": True,
    "reviewed_filesystem_capability": True,
    "openbao_implemented": False,
}
EXPECTED_EXTERNAL_GATES = [
    "official-provider-accounts-and-disposable-resources",
    "native-provider-cli-browser-and-credential-store",
    "native-cluster-and-exec-plugin-lifecycle",
    "native-visual-accessibility-and-resource-evidence",
    "signed-packaged-artifact-identity",
]
REQUIRED_SOURCE_TOKENS = {
    "extensions/devops-aws/src/lib.rs": {
        "MAX_CONFIG_BYTES: usize = 1024 * 1024",
        "MAX_PROFILES: usize = 128",
        "DuplicateEntry",
        "sso_session_section_name",
        'scope(context, "sso_region").ok_or_else',
        'format!("oidc.{region}.amazonaws.com")',
        "from_json_slice_without_duplicate_keys",
        '"--dry-run".into()',
        "SESSION_MANAGER_PLUGIN_ID",
        "execution_enabled: false",
    },
    "extensions/devops-azure/src/lib.rs": {
        "MAX_ACCOUNT_JSON_BYTES: usize = 256 * 1024",
        "MAX_ACCOUNTS: usize = 128",
        "from_json_slice_without_duplicate_keys",
        "CliDefaultInteractive",
        "cfg!(windows)",
        '"--use-device-code".into()',
        '"--subscription".into()',
        "requires_private_transient_file: true",
        "execution_enabled: false",
    },
    "extensions/devops-gcp/src/lib.rs": {
        "MAX_CONFIG_BYTES: usize = 256 * 1024",
        "MAX_SECTIONS: usize = 64",
        "MAX_ENTRIES: usize = 512",
        "GcpAdapterErrorCode::DuplicateEntry",
        "current_section = Some(section)",
        '"--configuration".into()',
        '"--no-launch-browser".into()',
        '"--tunnel-through-iap".into()',
        'private_environment_name: "KUBECONFIG".into()',
        "execution_enabled: false",
    },
    "extensions/devops-kubernetes/src/implementation.rs": {
        "MAX_KUBECONFIG_BYTES: usize = 1024 * 1024",
        "MAX_KUBECONFIG_SOURCES: usize = 16",
        "MAX_KUBECONFIG_ITEMS: usize = 256",
        "serde_saphyr::MergeKeyPolicy::Error",
        "max_aliases: 64",
        "max_depth: 32",
        "read_bounded_regular",
        "sensitive_exec_argument",
        "execution_enabled: false",
    },
    "extensions/devops-openshift/src/implementation.rs": {
        "default_enabled: false",
        "Capability::FilesystemRead",
        '"--web".into()',
        '"--namespace".into()',
        "execution_enabled: false",
    },
    "extensions/devops-teleport/src/lib.rs": {
        "MAX_STATUS_BYTES: usize = 256 * 1024",
        "MAX_PROFILES: usize = 32",
        "from_json_slice_without_duplicate_keys",
        '"TELEPORT_RELAY"',
        '"--add-keys-to-agent=no".into()',
        '"--relogin=false".into()',
        '"--request-mode=off".into()',
        "execution_enabled: false",
    },
    "apps/automexia-terminal/src/automexia/connections/providers.rs": {
        "MAX_PRODUCT_PROVIDERS: usize = 6",
        "ProviderKind::Aws",
        "ProviderKind::Azure",
        "ProviderKind::Gcp",
        "ProviderKind::Kubernetes",
        "ProviderKind::OpenShift",
        "ProviderKind::Teleport",
        "ProviderKind::OpenBao",
        "ProviderProductErrorCode::UnsupportedProvider",
        "publication: Option<ProviderProductPublication>",
        "ProviderProductErrorCode::StalePublication",
        "ProviderProductErrorCode::CrossSessionPublication",
    },
    "apps/automexia-terminal/src/automexia/connections/provider_transients.rs": {
        "MAX_PROVIDER_TRANSIENTS: usize = 16",
        "Duration::from_secs(24 * 60 * 60)",
        "parse_private_transient_source",
        "read_bounded_regular",
        "reserve_manager_root_with",
        "record.binding.capsule_revision != capsule_revision",
        "revalidate",
        "cleanup_expired",
        "disable_provider",
        "revoke_session",
        "shutdown",
        "impl Drop for ProviderTransientManager",
    },
    "apps/automexia-terminal/src/automexia/private_fs.rs": {
        "windows_local_wide_path",
        "Prefix::Disk(_)",
        "Prefix::VerbatimDisk(_)",
        "if *unit == b'/' as u16",
        "segment == [b'.' as u16]",
        "SetNamedSecurityInfoW",
        "GetNamedSecurityInfoW",
        "PROTECTED_DACL_SECURITY_INFORMATION",
    },
}
REQUIRED_TESTS = {
    "extensions/devops-aws/src/lib.rs": {
        "duplicate_profile_and_oversized_input_fail_closed",
        "sso_login_uses_the_bound_sso_region_and_never_the_service_region",
        "sts_output_is_strict_bounded_public_evidence",
    },
    "extensions/devops-azure/src/lib.rs": {
        "cli_version_floors_and_failure_states_are_explicit",
        "sensitive_duplicate_and_hostile_account_metadata_fail_closed",
        "login_and_status_are_exact_capsule_scoped_and_do_not_mutate_defaults",
        "aks_requires_an_opaque_private_transient_output_and_never_names_user_config",
    },
    "extensions/devops-gcp/src/lib.rs": {
        "credential_external_config_duplicate_and_bounds_fail_closed",
        "user_login_and_project_observation_are_exactly_scoped_without_global_mutation",
        "gke_uses_only_m11_private_kubeconfig_environment_binding",
    },
    "extensions/devops-kubernetes/tests/contracts.rs": {
        "exact_source_review_rejects_relative_oversize_and_changed_input",
        "public_parser_is_bounded_redacted_and_exec_denied_by_default",
        "json_cloud_sources_are_isolated_and_secret_exec_flags_fail_closed",
        "credential_redaction_canaries_are_present_in_the_actual_fixture",
        "every_inline_credential_is_opaque_in_public_and_merged_projections",
        "rejected_credential_documents_return_only_stable_redacted_errors",
    },
    "extensions/devops-openshift/tests/contracts.rs": {
        "web_login_is_visible_isolated_and_nonactivated",
        "project_inspection_and_rsh_never_mutate_global_project",
    },
    "extensions/devops-teleport/tests/contracts.rs": {
        "hostile_oversized_ambient_and_secret_status_fail_closed",
        "login_flows_are_exact_capsule_scoped_and_leave_mfa_to_tsh",
        "ssh_is_exact_nonactivated_and_disables_surprise_auth_or_access_requests",
    },
    "apps/automexia-terminal/tests/m8_m12_provider_product.rs": {
        "provider_catalog_review_and_stale_snapshot_stay_nonexecuting",
        "provider_navigation_is_consumed_by_the_hub_and_openbao_is_rejected",
    },
    "apps/automexia-terminal/src/automexia/connections/provider_transients.rs": {
        "publish_revalidate_and_revoke_keep_path_and_secrets_private",
        "every_m8_m12_kubeconfig_relation_uses_the_same_private_lifecycle",
        "capacity_disable_and_repeated_shutdown_are_bounded",
        "manager_root_reservation_never_adopts_a_preexisting_directory",
        "long_local_paths_publish_revalidate_and_revoke_real_provider_transients",
    },
    "apps/automexia-terminal/src/automexia/private_fs.rs": {
        "long_local_connection_paths_pass_native_acl_revalidation",
        "native_acl_path_conversion_rejects_relative_remote_and_dot_segments",
    },
}
REQUIRED_FUZZ_TOKENS = {
    "automexia_devops_aws::parse_public_config",
    "automexia_devops_azure::parse_public_accounts",
    "automexia_devops_gcp::parse_public_configuration",
    "automexia_devops_kubernetes::parse_private_transient_source",
    "automexia_devops_teleport::parse_public_status",
}
REQUIRED_FUZZ_MANIFEST_TOKENS = {
    'multicloud-provider-inputs = [',
    'required-features = ["multicloud-provider-inputs"]',
    'automexia-devops-aws = { path = "../extensions/devops-aws", optional = true }',
    'automexia-terminal = { path = "../apps/automexia-terminal", default-features = false, optional = true }',
}
REQUIRED_BENCHMARK_TOKENS = {
    "extensions/devops-aws/benches/provider.rs": "aws_public_config_128_profiles_and_sso_sessions",
    "extensions/devops-azure/benches/provider.rs": "azure_public_accounts_128",
    "extensions/devops-gcp/benches/provider.rs": "gcp_public_configuration_near_limit",
    "extensions/devops-kubernetes/benches/kubeconfig.rs": "parse 900 KiB kubeconfig",
    "extensions/devops-teleport/benches/status.rs": "teleport_public_status",
}
FORBIDDEN_ADAPTER_PRIMITIVES = {
    "std::process",
    "command::new",
    "std::net",
    "tcpstream",
    "tcplistener",
    "tokio::process",
    "tokio::net",
    "reqwest",
    "unsafe {",
}
REQUIRED_NATIVE_SCENARIO = "connection-hub-providers-review"
REQUIRED_RESOURCE_SCENARIO = "connection-hub-providers-replacement"
REQUIRED_VISUAL_SURFACE = "connection-hub-providers-review"
REQUIRED_ACCESSIBILITY_TASK = "connection-hub-providers-review"
REQUIRED_VISUAL_CAPTURE_COUNT = 9_504


class MultiCloudContractError(ValueError):
    """The M8-M12 contract or one of its evidence owners drifted."""


def bounded_text(path: Path, maximum: int = MAX_EVIDENCE_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise MultiCloudContractError(f"required regular file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise MultiCloudContractError(f"M8-M12 evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise MultiCloudContractError(f"M8-M12 evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise MultiCloudContractError(f"duplicate M8-M12 contract key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    return json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = load_json(path)
    expected_keys = {
        "schema", "phase", "status", "providers", "limits", "authorities",
        "external_gates", "source_files", "test_files", "fuzz_target",
        "benchmarks", "s1_policy", "documents",
    }
    if not isinstance(document, dict) or set(document) != expected_keys:
        raise MultiCloudContractError("M8-M12 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1,
        "M8-M12",
        "implemented-nonactivated-external-provider-native-release-evidence-pending",
    ):
        raise MultiCloudContractError("M8-M12 contract identity changed")
    if document["providers"] != EXPECTED_PROVIDERS:
        raise MultiCloudContractError("M8-M12 provider set changed")
    if document["limits"] != EXPECTED_LIMITS:
        raise MultiCloudContractError("M8-M12 limits changed")
    if document["authorities"] != EXPECTED_AUTHORITIES:
        raise MultiCloudContractError("M8-M12 authority boundary changed")
    if document["external_gates"] != EXPECTED_EXTERNAL_GATES:
        raise MultiCloudContractError("M8-M12 external gates changed")
    for key in ("source_files", "test_files", "benchmarks", "documents"):
        values = document[key]
        if not isinstance(values, list) or not values or len(values) != len(set(values)):
            raise MultiCloudContractError(f"M8-M12 {key} must contain unique entries")
    if set(document["source_files"]) != set(REQUIRED_SOURCE_TOKENS):
        raise MultiCloudContractError("M8-M12 source ownership changed")
    if set(document["test_files"]) != set(REQUIRED_TESTS):
        raise MultiCloudContractError("M8-M12 test ownership changed")
    if set(document["benchmarks"]) != set(REQUIRED_BENCHMARK_TOKENS):
        raise MultiCloudContractError("M8-M12 benchmark ownership changed")
    return document


def require_tokens(relative: str, tokens: set[str], root: Path) -> str:
    source = bounded_text(root / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise MultiCloudContractError(f"{relative} is missing M8-M12 evidence: {missing}")
    return source


def validate_s1(relative: str, root: Path) -> None:
    policy = load_json(root / relative)
    suites = policy.get("required_suites") if isinstance(policy, dict) else None
    if not isinstance(suites, list):
        raise MultiCloudContractError("S1 policy has no bounded required_suites")
    found = {"native": 0, "resource": 0, "visual": 0, "accessibility": 0}
    for suite in suites:
        domain = suite.get("domain")
        coverage = suite.get("coverage", {})
        if domain == "native":
            found["native"] += 1
            if REQUIRED_NATIVE_SCENARIO not in coverage.get("scenarios", []):
                raise MultiCloudContractError("native provider S1 scenario is missing")
        if suite.get("tool") == "native-resource":
            found["resource"] += 1
            if REQUIRED_RESOURCE_SCENARIO not in coverage.get("scenarios", []):
                raise MultiCloudContractError("resource provider S1 scenario is missing")
        if domain == "visual":
            found["visual"] += 1
            if REQUIRED_VISUAL_SURFACE not in coverage.get("surfaces", []):
                raise MultiCloudContractError("visual provider S1 surface is missing")
            if coverage.get("capture_count") != REQUIRED_VISUAL_CAPTURE_COUNT:
                raise MultiCloudContractError("visual provider S1 capture count changed")
        if domain == "accessibility":
            found["accessibility"] += 1
            if REQUIRED_ACCESSIBILITY_TASK not in coverage.get("tasks", []):
                raise MultiCloudContractError("accessibility provider S1 task is missing")
    if any(count == 0 for count in found.values()):
        raise MultiCloudContractError(f"provider S1 suite classes are missing: {found}")


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    contract = load_contract(root / CONTRACT.relative_to(ROOT))
    sources = {
        relative: require_tokens(relative, tokens, root)
        for relative, tokens in REQUIRED_SOURCE_TOKENS.items()
    }
    adapter_text = "\n".join(
        sources[relative].split("#[cfg(test)]", 1)[0]
        for relative in contract["source_files"]
        if relative.startswith("extensions/")
    ).casefold()
    forbidden = next(
        (token for token in sorted(FORBIDDEN_ADAPTER_PRIMITIVES) if token in adapter_text),
        None,
    )
    if forbidden:
        raise MultiCloudContractError(f"M8-M12 adapter owns forbidden authority: {forbidden}")
    for relative, names in REQUIRED_TESTS.items():
        source = bounded_text(root / relative)
        missing = sorted(name for name in names if f"fn {name}(" not in source)
        if missing:
            raise MultiCloudContractError(f"{relative} is missing M8-M12 tests: {missing}")
    require_tokens(contract["fuzz_target"], REQUIRED_FUZZ_TOKENS, root)
    require_tokens("fuzz/Cargo.toml", REQUIRED_FUZZ_MANIFEST_TOKENS, root)
    for relative, token in REQUIRED_BENCHMARK_TOKENS.items():
        require_tokens(relative, {token, "criterion_group!"}, root)
    require_tokens(
        "extensions/devops-kubernetes/benches/kubeconfig.rs",
        {"parse empty kubeconfig", "parse 256 credential users",
         "reject oversized kubeconfig", "reject malformed kubeconfig"},
        root,
    )
    validate_s1(contract["s1_policy"], root)
    for relative in contract["documents"]:
        source = bounded_text(root / relative)
        normalized = source.upper()
        if not any(anchor in normalized for anchor in ("M8", "M9", "M10", "M11", "M12")):
            raise MultiCloudContractError(f"{relative} does not identify M8-M12")
    require_tokens(
        ".github/workflows/ci.yml",
        {"python tools/ci/check_m8_m12_multicloud.py", "python tools/ci/test_m8_m12_multicloud.py"},
        root,
    )
    require_tokens(
        ".github/workflows/nightly.yml",
        {
            "multicloud_provider_inputs",
            "--no-default-features --features multicloud-provider-inputs",
        },
        root,
    )
    require_tokens(
        "tools/ci/qa.py",
        {"tools/ci/check_m8_m12_multicloud.py", "tools/ci/test_m8_m12_multicloud.py"},
        root,
    )
    return {
        "sources": len(REQUIRED_SOURCE_TOKENS),
        "tests": sum(len(names) for names in REQUIRED_TESTS.values()),
        "limits": len(EXPECTED_LIMITS),
        "providers": len(EXPECTED_PROVIDERS),
        "benchmarks": len(REQUIRED_BENCHMARK_TOKENS),
    }


def main() -> int:
    try:
        counts = validate_repository()
    except (MultiCloudContractError, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"M8-M12 multi-cloud validation failed: {error}", file=sys.stderr)
        return 1
    print(f"PASS: M8-M12 nonactivated multi-cloud contract is frozen ({counts})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
