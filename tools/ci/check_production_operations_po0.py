#!/usr/bin/env python3
"""Validate the proposed, non-activating Production Operations PO0 contract."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = Path(
    "tests/fixtures/production-operations/po0-contract-v1.json"
)
SPEC_PATH = Path("docs/SITUATION-AWARE-PRODUCTION-OPERATIONS.md")
CONTRACT_DOC_PATH = Path(
    "docs/SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md"
)
UX_PATH = Path("docs/SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md")
TESTING_PATH = Path("docs/SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md")
ADR_PATH = Path("docs/adr/0034-situation-aware-production-operations.md")
ROADMAP_PATH = Path("docs/ROADMAP.md")
AUDIT_PATH = Path("docs/PHASE-IMPLEMENTATION-AUDIT.md")
MAX_CONTRACT_BYTES = 512 * 1024
MAX_DOCUMENT_BYTES = 2 * 1024 * 1024
EXPECTED_CANONICAL_SHA256 = (
    "8038ce24aa0f4223a910ed2293f9c40b17882e617fda9a01a14d13f65d7004cf"
)


class ProductionOperationsContractError(ValueError):
    """The PO0 planning contract is missing, weakened, or activating."""


def _bounded_text(path: Path, maximum: int, label: str) -> str:
    data = path.read_bytes()
    if not data or len(data) > maximum:
        raise ProductionOperationsContractError(
            f"{label} must contain 1..{maximum} bytes"
        )
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ProductionOperationsContractError(
            f"{label} must be valid UTF-8"
        ) from error


def _pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ProductionOperationsContractError(
                f"duplicate key {key!r} is forbidden"
            )
        result[key] = value
    return result


def parse_contract(text: str) -> Any:
    try:
        return json.loads(text, object_pairs_hook=_pairs)
    except (json.JSONDecodeError, UnicodeError) as error:
        raise ProductionOperationsContractError(
            f"PO0 contract is not strict JSON: {error}"
        ) from error


def canonical_digest(document: Any) -> str:
    encoded = json.dumps(
        document, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _exact_keys(
    value: Any, expected: set[str], label: str
) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ProductionOperationsContractError(f"{label} must be an object")
    actual = set(value)
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        raise ProductionOperationsContractError(
            f"{label} keys changed; missing={missing}, extra={extra}"
        )
    return value


def _unique_strings(
    value: Any, label: str, *, minimum: int = 1, exact_count: int | None = None
) -> list[str]:
    if not isinstance(value, list):
        raise ProductionOperationsContractError(f"{label} must be an array")
    if len(value) < minimum or (exact_count is not None and len(value) != exact_count):
        raise ProductionOperationsContractError(
            f"{label} has an invalid entry count"
        )
    if (
        any(not isinstance(item, str) or not item or len(item.encode("utf-8")) > 256 for item in value)
        or len(set(value)) != len(value)
    ):
        raise ProductionOperationsContractError(
            f"{label} must contain unique bounded strings"
        )
    return value


EXPECTED_TOP_LEVEL = {
    "schema",
    "phase",
    "status",
    "authority",
    "ownership",
    "formats",
    "record_kinds",
    "common_envelope",
    "record_payloads",
    "knowledge_states",
    "environment_classes",
    "environment_classification_precedence",
    "evidence_quality",
    "freshness_seconds",
    "hard_gates",
    "ranking",
    "kubernetes_rollout_rules",
    "policy",
    "operation_states",
    "operation_invariants",
    "provider_profiles",
    "action_profiles",
    "collection_strategy",
    "planned_configuration",
    "planned_actions",
    "journal",
    "limits",
    "accessibility",
    "verification",
    "traceability",
    "external_gates",
}

EXPECTED_AUTHORITY = {
    "adr": "0034",
    "accepted": False,
    "runtime_activation": False,
    "provider_requests": False,
    "watchers": False,
    "completion_source": False,
    "settings": False,
    "shortcuts": False,
    "journal": False,
    "managed_execution": False,
    "managed_sessions": False,
    "organization_packs": False,
    "local_model": False,
    "new_dependencies": "none",
    "fallback": "cp1-native-completion",
}

EXPECTED_FORMATS = {
    "internal": "bounded-typed-rust",
    "interchange": "strict-json-schema-2020-12",
    "duplicate_keys": "reject",
    "unknown_major": "reject",
    "unknown_fields": "reject-unless-versioned-extension-map",
    "canonicalization": "rfc8785-jcs",
    "digest": "sha256",
    "time": "rfc3339-utc-with-source-and-ingress-time",
    "version": "semver-2.0.0",
    "user_configuration": "existing-config-toml-kebab-case",
    "planning_fixture_is_runtime_input": False,
}

EXPECTED_RECORD_KINDS = {
    "environment-passport",
    "observation",
    "evidence-snapshot",
    "change-record",
    "field-ownership-record",
    "resource-explanation",
    "comparison-finding",
    "network-path-assessment",
    "slo-impact-summary",
    "situation-assessment",
    "candidate-action",
    "policy-decision",
    "impact-preflight",
    "managed-action-grant",
    "operation-monitor",
    "managed-diagnostic-session",
    "action-receipt",
}

EXPECTED_ENVELOPE = {
    "schema-version",
    "kind",
    "record-id",
    "route-id",
    "environment-revision",
    "generation",
    "source-id",
    "source-version",
    "observed-at",
    "received-at",
    "expires-at",
    "source-sequence",
    "coverage",
    "clock-quality",
    "redaction-class",
    "canonical-sha256",
}

EXPECTED_RECORD_PAYLOADS = {
    "environment-passport": frozenset(
        """
        environment-class classification-source provider profile-reference
        account-subscription-project-tenant region-zone
        cluster-server-origin-fingerprint cluster-uid namespace-project-scope
        effective-identity-fingerprint identity-groups-role-summary
        authentication-expiry-state jit-elevation-state
        gitops-owner-revision-state incident-reference change-window-state
        passport-fields lock-generation passport-digest
        """.split()
    ),
    "observation": frozenset(
        """
        subject-reference predicate-id typed-value value-schema scope time-window
        quality provider-native-reference
        """.split()
    ),
    "evidence-snapshot": frozenset(
        """
        passport-digest observation-references resource-node-references
        resource-edge-references supporting-evidence contradicting-evidence
        missing-sources snapshot-quality snapshot-digest
        """.split()
    ),
    "change-record": frozenset(
        """
        resource-reference change-kind actor-public-reference
        provider-event-reference changed-semantic-fields before-digest
        after-digest resolved-revision correlation-window causal-authority
        """.split()
    ),
    "field-ownership-record": frozenset(
        """
        resource-reference semantic-field-id manager-public-reference owner-kind
        operation-kind desired-source-reference last-change-reference
        ownership-quality
        """.split()
    ),
    "resource-explanation": frozenset(
        """
        resource-reference state-class reason-code current-semantic-state
        desired-semantic-state blockers owner-chain dependency-references
        supporting-evidence contradictions unknowns
        """.split()
    ),
    "comparison-finding": frozenset(
        """
        subject-reference baseline-kind cohort-definition baseline-references
        normalized-field-ids equal-fields different-fields unknown-fields
        revision-mapping coverage finding-state
        """.split()
    ),
    "network-path-assessment": frozenset(
        """
        origin-vantage destination-reference protocol-port ordered-layers
        layer-states evidence-references untested-layers active-probe-used
        path-conclusion
        """.split()
    ),
    "slo-impact-summary": frozenset(
        """
        service-reference objective-reference evaluation-window
        recording-rule-reference burn-rate-windows request-volume
        missing-evaluations low-traffic-state impact-class root-cause-authority
        """.split()
    ),
    "situation-assessment": frozenset(
        """
        operational-intent subject-references finding-references hypotheses
        supporting-evidence contradictions missing-evidence eligible-rule-ids
        assessment-state
        """.split()
    ),
    "candidate-action": frozenset(
        """
        action-id tool-profile-id effect-class typed-operation target-uids
        safety-class reason-message-id evidence-references hard-gate-results
        refusal-reasons verification-profile-id recovery-profile-id ranking-key
        candidate-digest
        """.split()
    ),
    "policy-decision": frozenset(
        """
        decision-state policy-source policy-revision decision-id subject-digest
        action-digest target-digest environment-digest obligations
        decision-expiry decision-coverage
        """.split()
    ),
    "impact-preflight": frozenset(
        """
        passport-digest candidate-digest evidence-snapshot-digest
        exact-executable-or-api-operation exact-arguments-or-typed-request
        target-uids target-count dependency-impact authorization-result
        admission-limitations gitops-state policy-decisions
        jit-credential-and-approval-state change-window-state verification-plan
        stabilization-window timeout recovery-plan preflight-digest
        """.split()
    ),
    "managed-action-grant": frozenset(
        """
        operation-id route-passport-evidence-policy-preflight-digests
        exact-executable-or-api-operation exact-arguments-or-typed-request
        environment-allowlist target-uids capability-principal
        authorization-reference target-count-ceiling timeout
        verification-profile-id recovery-profile-id broker-generation expires-at
        consumption-state
        """.split()
    ),
    "operation-monitor": frozenset(
        """
        operation-id operation-state exact-target-summary started-at elapsed-time
        deadline before-state-reference last-observation-reference
        stabilization-state verification-state cleanup-state
        cancel-stop-availability failure-or-uncertainty-reason recovery-reference
        """.split()
    ),
    "managed-diagnostic-session": frozenset(
        """
        session-id session-kind operation-id target-uids endpoint-or-vantage
        protocol-port immutable-image-or-profile
        authorization-and-admission-references traffic-and-time-limits
        listener-process-and-temporary-resource-owners detach-policy cleanup-plan
        cleanup-verification
        """.split()
    ),
    "action-receipt": frozenset(
        """
        operation-id route-and-passport-digest action-and-rule-ids
        target-public-identity-digest environment-and-risk-class
        provider-and-profile-versions preflight-policy-and-grant-digests
        public-decision-ids start-and-end-time final-state
        cancellation-or-timeout-reason verification-and-recovery-profile-ids
        verification-and-recovery-outcomes cleanup-ownership-references
        """.split()
    ),
}

EXPECTED_ACTION_PROFILES = {
    "kubernetes": frozenset(
        """
        kubernetes.context.refresh:PO1:read-only:kubernetes-api-and-reviewed-context
        kubernetes.workload.explain:PO2:read-only:kubernetes-api
        kubernetes.resource.compare-healthy:PO2:read-only:kubernetes-api-and-reviewed-cohort
        kubernetes.network.explain:PO2:read-only:kubernetes-api-and-declared-vantage
        kubernetes.rollout.status:PO3:read-only:kubectl-or-kubernetes-api
        kubernetes.rollout.history:PO3:read-only:kubectl-or-kubernetes-api
        kubernetes.rollout.undo:PO4:reviewed-mutation:kubernetes-api-and-gitops-policy
        kubernetes.rollout.restart:PO4:reviewed-mutation:kubernetes-api-and-gitops-policy
        kubernetes.port-forward:PO6:managed-session:kubernetes-subresource-and-existing-broker
        kubernetes.debug-workload:PO6:managed-session:kubernetes-api-admission-and-existing-broker
        """.split()
    ),
    "openshift": frozenset(
        """
        openshift.context.refresh:PO1:read-only:openshift-api-and-reviewed-kube-context
        openshift.workload.explain:PO2:read-only:openshift-and-kubernetes-apis
        openshift.resource.compare-healthy:PO2:read-only:openshift-and-kubernetes-apis-and-reviewed-cohort
        openshift.rollout.status:PO3:read-only:oc-or-compatible-kubernetes-api
        """.split()
    ),
    "aws": frozenset(
        """
        aws.identity.refresh:PO1:read-only:sts-get-caller-identity
        aws.changes.lookup:PO2:read-only:cloudtrail-bounded-event-history
        aws.permission.review:PO4:advisory:iam-simulation-and-live-service-authority
        """.split()
    ),
    "azure": frozenset(
        """
        azure.identity.refresh:PO1:read-only:azure-resource-manager-public-context
        azure.changes.lookup:PO2:read-only:azure-activity-log
        azure.permission.review:PO4:advisory:azure-rbac-effective-evaluation
        azure.deployment.what-if:PO4:simulation:azure-resource-manager-what-if
        """.split()
    ),
    "gcp": frozenset(
        """
        gcp.identity.refresh:PO1:read-only:explicit-gcloud-configuration
        gcp.changes.lookup:PO2:read-only:cloud-audit-logs
        gcp.asset-history.lookup:PO2:read-only:cloud-asset-inventory
        gcp.permission.review:PO4:advisory:test-iam-permissions-and-live-service-authority
        """.split()
    ),
    "gitops_iac": frozenset(
        """
        gitops.desired-live.diff:PO2:read-only:argo-or-flux-native-diff
        gitops.reconciliation.status:PO2:read-only:argo-or-flux-controller
        gitops.sync-window.review:PO4:advisory:gitops-controller
        gitops.reconcile:PO6:managed-mutation:gitops-controller-and-existing-broker
        helm.release.status:PO2:read-only:helm-release-state
        helm.release.simulate-upgrade:PO4:simulation:helm-client-or-server-dry-run
        terraform.plan-summary.inspect:PO2:read-only:approved-redacted-plan-summary
        """.split()
    ),
    "policy_observability": frozenset(
        """
        policy.decision.refresh:PO4:read-only:external-policy-engine
        identity.elevation.status:PO4:read-only:external-jit-provider
        observability.slo-impact:PO2:read-only:reviewed-recording-or-slo-rule
        observability.log-summary:PO2:read-only:bounded-log-adapter
        observability.trace-correlate:PO2:correlation-only:trace-backend
        """.split()
    ),
}

EXPECTED_FRESHNESS = {
    "context_normal": 60,
    "context_production": 15,
    "target_health_normal": 15,
    "target_health_production": 5,
    "authorization_normal": 30,
    "authorization_production": 10,
    "policy_normal": 30,
    "policy_production": 10,
    "gitops_normal": 30,
    "gitops_production": 15,
    "watch_normal": 15,
    "watch_production": 5,
    "change_collection": 60,
    "slo_normal_max": 120,
    "slo_production_max": 60,
    "log_summary_normal": 30,
    "log_summary_production": 10,
    "organization_metadata_normal_max": 86400,
    "organization_metadata_production_max": 3600,
    "preflight_authorization_completion": 5,
    "one_use_grant": 15,
}

EXPECTED_LIMITS = {
    "typing_provider_filesystem_process_or_network_operations": 0,
    "active_requests_per_pane": 1,
    "queued_latest_generations_per_pane": 1,
    "active_routes_global": 32,
    "candidates_per_request": 32,
    "visible_situation_rows": 5,
    "evidence_references_per_candidate": 8,
    "display_field_bytes": 4096,
    "request_frame_bytes": 65536,
    "reply_frame_bytes": 262144,
    "explicit_scopes": 4,
    "resource_nodes": 10000,
    "snapshot_cache_bytes": 33554432,
    "evidence_reads_global": 4,
    "evidence_reads_per_adapter": 2,
    "queries_per_second_default": 5,
    "query_burst_default": 10,
    "comparison_resources": 32,
    "comparison_revisions": 4,
    "comparison_environments": 4,
    "live_log_sources_per_view": 8,
    "live_log_sources_global": 16,
    "live_log_records_per_view": 20000,
    "live_log_bytes_per_view": 8388608,
    "live_log_bytes_per_second_per_view": 1048576,
    "active_probes_per_route": 1,
    "active_probes_global": 2,
    "active_probe_deadline_seconds": 30,
    "kubernetes_port_forwards_per_route": 4,
    "debug_sessions_per_route": 1,
    "managed_sessions_global": 8,
    "session_receipts": 256,
    "session_journal_bytes": 2097152,
    "persistent_receipts": 4096,
    "persistent_journal_bytes": 16777216,
    "persistent_retention_days_max": 30,
    "read_only_retries": 2,
    "mutation_retries": 0,
}


def _validate_ownership(document: dict[str, Any]) -> None:
    ownership = _exact_keys(
        document["ownership"],
        {
            "pure_model",
            "devops_rules",
            "provider_adapters",
            "application",
            "external_authorities",
            "forbidden_duplicate_owners",
        },
        "ownership",
    )
    for key, value in ownership.items():
        _unique_strings(value, f"ownership.{key}")
    forbidden = set(ownership["forbidden_duplicate_owners"])
    required = {
        "pty",
        "terminal-grid",
        "shell-editor",
        "completion-popup",
        "provider-credential-store",
        "process-runner",
        "renderer",
        "session",
        "package-trust",
    }
    if forbidden != required:
        raise ProductionOperationsContractError(
            "duplicate-owner denials changed"
        )


def _validate_evidence(document: dict[str, Any]) -> None:
    if set(_unique_strings(document["record_kinds"], "record_kinds", exact_count=17)) != EXPECTED_RECORD_KINDS:
        raise ProductionOperationsContractError("record kinds changed")
    if set(_unique_strings(document["common_envelope"], "common_envelope", exact_count=16)) != EXPECTED_ENVELOPE:
        raise ProductionOperationsContractError("common envelope changed")
    if document["knowledge_states"] != [
        "observed",
        "derived",
        "correlated",
        "policy",
        "conflicting",
        "unknown",
    ]:
        raise ProductionOperationsContractError("knowledge states changed")
    if document["environment_classes"] != [
        "production",
        "staging",
        "development",
        "sandbox",
        "unknown",
    ]:
        raise ProductionOperationsContractError("environment classes changed")
    classification = _unique_strings(
        document["environment_classification_precedence"],
        "environment classification",
        exact_count=5,
    )
    if classification[-1] != "unknown-treated-as-production-for-mutation":
        raise ProductionOperationsContractError(
            "unknown environment no longer fails closed"
        )
    quality = _exact_keys(
        document["evidence_quality"],
        {
            "dimensions",
            "eligibility",
            "opaque_average_score",
            "correlation_is_causation",
            "unknown_mutation_behavior",
        },
        "evidence quality",
    )
    if (
        quality["dimensions"]
        != [
            "authority",
            "provenance",
            "freshness",
            "coverage",
            "directness",
            "clock",
            "gaps",
        ]
        or quality["eligibility"] != ["eligible", "degraded", "ineligible"]
        or quality["opaque_average_score"] is not False
        or quality["correlation_is_causation"] is not False
        or quality["unknown_mutation_behavior"] != "block-production"
    ):
        raise ProductionOperationsContractError(
            "evidence quality or uncertainty behavior changed"
        )
    if document["freshness_seconds"] != EXPECTED_FRESHNESS:
        raise ProductionOperationsContractError("freshness ceilings changed")


def _validate_record_payloads(document: dict[str, Any]) -> None:
    payloads = _exact_keys(
        document["record_payloads"],
        set(EXPECTED_RECORD_PAYLOADS),
        "record payloads",
    )
    for kind, expected in EXPECTED_RECORD_PAYLOADS.items():
        actual = set(
            _unique_strings(
                payloads[kind],
                f"record payload {kind}",
                exact_count=len(expected),
            )
        )
        if actual != expected:
            raise ProductionOperationsContractError(
                f"record payload {kind} changed"
            )


def _validate_decisions(document: dict[str, Any]) -> None:
    _unique_strings(document["hard_gates"], "hard_gates", exact_count=9)
    ranking = _exact_keys(
        document["ranking"],
        {
            "algorithm",
            "dimensions",
            "read_only_precedes_mutation_when_otherwise_equal",
            "diagnosis_precedes_symptom_masking",
            "business_priority_requires_reviewed_source",
            "automatic_execution",
        },
        "ranking",
    )
    if (
        ranking["algorithm"] != "stable-lexicographic"
        or len(_unique_strings(ranking["dimensions"], "ranking dimensions")) != 10
        or ranking["read_only_precedes_mutation_when_otherwise_equal"] is not True
        or ranking["diagnosis_precedes_symptom_masking"] is not True
        or ranking["business_priority_requires_reviewed_source"] is not True
        or ranking["automatic_execution"] is not False
    ):
        raise ProductionOperationsContractError("ranking safety changed")
    rollout = set(
        _unique_strings(
            document["kubernetes_rollout_rules"],
            "kubernetes rollout rules",
            exact_count=12,
        )
    )
    for required in {
        "image-pull-restart-refused",
        "pending-unschedulable-restart-refused",
        "crashloop-is-symptom-not-root-cause",
        "stale-conflicting-or-owner-unknown-refuses-mutation",
    }:
        if required not in rollout:
            raise ProductionOperationsContractError(
                f"Kubernetes rollout policy lost {required}"
            )

    policy = _exact_keys(
        document["policy"],
        {
            "states",
            "precedence",
            "deny_wins",
            "unknown_blocks_production_mutation",
            "lower_level_may_override_deny",
            "embedded_policy_runtime",
            "input_or_result_persisted",
        },
        "policy",
    )
    if (
        policy["states"] != ["allow", "deny", "unknown"]
        or policy["precedence"][0] != "automexia-hard-safety"
        or policy["deny_wins"] is not True
        or policy["unknown_blocks_production_mutation"] is not True
        or policy["lower_level_may_override_deny"] is not False
        or policy["embedded_policy_runtime"] is not False
        or policy["input_or_result_persisted"] is not False
    ):
        raise ProductionOperationsContractError("policy precedence changed")

    states = set(_unique_strings(document["operation_states"], "operation states"))
    for required in {
        "revalidating",
        "stabilizing",
        "verifying",
        "uncertain",
        "recovery-proposed",
        "complete",
    }:
        if required not in states:
            raise ProductionOperationsContractError(
                f"operation lifecycle lost {required}"
            )
    invariants = set(
        _unique_strings(document["operation_invariants"], "operation invariants")
    )
    for required in {
        "process-exit-is-not-success",
        "no-automatic-mutation-retry",
        "no-automatic-recovery",
        "no-second-mutation-chain",
    }:
        if required not in invariants:
            raise ProductionOperationsContractError(
                f"operation lifecycle lost {required}"
            )


def _validate_providers(document: dict[str, Any]) -> None:
    profiles = _exact_keys(
        document["provider_profiles"],
        {
            "kubernetes",
            "openshift",
            "aws",
            "azure",
            "gcp",
            "gitops_iac",
            "policy_observability",
        },
        "provider profiles",
    )
    for key, value in profiles.items():
        _unique_strings(value, f"provider profile {key}")
    required_markers = {
        "kubernetes": {
            "self-subject-review-and-exact-self-subject-access-review",
            "bounded-list-watch-resource-version-relist-on-410",
            "no-kubectl-external-diff-or-show-secrets",
        },
        "aws": {
            "explicit-named-profile",
            "iam-simulation-advisory-only",
        },
        "azure": {
            "never-az-account-set",
            "arm-what-if-advisory-with-coverage-limits",
        },
        "gcp": {
            "never-activate-global-configuration",
            "test-iam-permissions-exact-resource-advisory",
        },
        "gitops_iac": {
            "reconcile-and-sync-are-mutations",
            "terraform-opentofu-raw-plan-and-state-never-ingested-or-persisted",
        },
    }
    for profile, markers in required_markers.items():
        if not markers.issubset(set(profiles[profile])):
            raise ProductionOperationsContractError(
                f"provider profile {profile} lost a safety rule"
            )

    collection = _exact_keys(
        document["collection_strategy"],
        {
            "one_shot",
            "sustained_kubernetes",
            "cloud_sdks",
            "polling",
            "typing_io",
            "startup_io",
            "watch_bookmark_is_object_freshness",
            "raw_logs_or_metric_series_cached",
            "raw_terraform_plan_or_state_ingested",
        },
        "collection strategy",
    )
    if (
        collection["one_shot"] != "existing-exact-argv-provider-adapters"
        or collection["sustained_kubernetes"]
        != "conditionally-adopt-kube-runtime-after-dependency-review"
        or collection["cloud_sdks"] != "not-adopted-by-default"
        or any(
            collection[key] is not False
            for key in (
                "typing_io",
                "startup_io",
                "watch_bookmark_is_object_freshness",
                "raw_logs_or_metric_series_cached",
                "raw_terraform_plan_or_state_ingested",
            )
        )
    ):
        raise ProductionOperationsContractError(
            "collection or hot-path boundary changed"
        )


def _validate_action_profiles(document: dict[str, Any]) -> None:
    profiles = _exact_keys(
        document["action_profiles"],
        set(EXPECTED_ACTION_PROFILES),
        "action profiles",
    )
    all_ids: set[str] = set()
    for profile, expected in EXPECTED_ACTION_PROFILES.items():
        entries = profiles[profile]
        if not isinstance(entries, list) or len(entries) != len(expected):
            raise ProductionOperationsContractError(
                f"action profile {profile} has an invalid entry count"
            )
        actual: set[str] = set()
        for entry in entries:
            item = _exact_keys(
                entry,
                {"id", "phase", "effect", "authority"},
                f"action profile {profile} entry",
            )
            if any(
                not isinstance(item[key], str)
                or not item[key]
                or len(item[key].encode("utf-8")) > 256
                for key in ("id", "phase", "effect", "authority")
            ):
                raise ProductionOperationsContractError(
                    f"action profile {profile} contains an invalid value"
                )
            if item["id"] in all_ids:
                raise ProductionOperationsContractError(
                    f"action ID {item['id']!r} is duplicated"
                )
            all_ids.add(item["id"])
            actual.add(
                ":".join(
                    (
                        item["id"],
                        item["phase"],
                        item["effect"],
                        item["authority"],
                    )
                )
            )
        if actual != expected:
            raise ProductionOperationsContractError(
                f"action profile {profile} changed"
            )


def _validate_planned_surface_and_storage(document: dict[str, Any]) -> None:
    config = _exact_keys(
        document["planned_configuration"],
        {
            "namespace",
            "enabled_default",
            "mode_default",
            "suggestions_default",
            "max_visible_situation_rows",
            "live_logs_default",
            "managed_actions_default",
            "incident_persistence_default",
            "persistent_retention_days_default",
            "port_forward_default",
            "active_probe_default",
            "debug_workload_default",
            "allow_detach_default",
            "loopback_only",
            "user_may_raise_safety_limits",
            "current_config_schema_contains_these_keys",
        },
        "planned configuration",
    )
    for key in (
        "enabled_default",
        "suggestions_default",
        "live_logs_default",
        "managed_actions_default",
        "port_forward_default",
        "active_probe_default",
        "debug_workload_default",
        "allow_detach_default",
        "user_may_raise_safety_limits",
        "current_config_schema_contains_these_keys",
    ):
        if config[key] is not False:
            raise ProductionOperationsContractError(
                f"planned setting {key} became enabled"
            )
    if (
        config["namespace"] != "production-operations"
        or config["mode_default"] != "read-only"
        or config["incident_persistence_default"] != "session"
        or config["persistent_retention_days_default"] != 0
        or config["loopback_only"] is not True
    ):
        raise ProductionOperationsContractError(
            "planned configuration safety changed"
        )

    actions = _exact_keys(
        document["planned_actions"],
        {
            "default_shortcut",
            "completion_invocation",
            "action_ids",
            "current_action_registry_contains_these_actions",
            "selection_can_execute_managed_mutation",
        },
        "planned actions",
    )
    if (
        actions["default_shortcut"] != "none"
        or actions["completion_invocation"] != "existing-cp5-explicit-invocation"
        or actions["current_action_registry_contains_these_actions"] is not False
        or actions["selection_can_execute_managed_mutation"] is not False
    ):
        raise ProductionOperationsContractError(
            "planned action or shortcut boundary changed"
        )
    _unique_strings(actions["action_ids"], "planned action IDs", exact_count=5)

    journal = _exact_keys(
        document["journal"],
        {
            "default",
            "persistent_default",
            "allowed",
            "forbidden",
            "storage",
            "export",
            "uninstall",
        },
        "journal",
    )
    if (
        journal["default"] != "memory-only"
        or journal["persistent_default"] is not False
        or journal["storage"] != "existing-private-no-follow-atomic-owner"
        or "credentials-tokens-certificates-or-secrets"
        not in _unique_strings(journal["forbidden"], "journal forbidden")
        or "raw-logs-or-metrics" not in journal["forbidden"]
        or "provider-responses" not in journal["forbidden"]
    ):
        raise ProductionOperationsContractError(
            "journal privacy or storage boundary changed"
        )
    _unique_strings(journal["allowed"], "journal allowed")


def _validate_assurance(document: dict[str, Any]) -> None:
    if document["limits"] != EXPECTED_LIMITS:
        raise ProductionOperationsContractError("resource ceilings changed")
    accessibility = _exact_keys(
        document["accessibility"],
        {
            "listbox_only_for_single_selectable_values",
            "focus_and_selection_separate",
            "tab_leaves_list",
            "modal_traps_focus_and_escape_closes",
            "modal_focus_returns_to_invoker",
            "least_destructive_action_first",
            "color_or_icon_only_meaning",
            "watch_events_announced_individually",
            "wcag",
        },
        "accessibility",
    )
    for key in (
        "listbox_only_for_single_selectable_values",
        "focus_and_selection_separate",
        "tab_leaves_list",
        "modal_traps_focus_and_escape_closes",
        "modal_focus_returns_to_invoker",
        "least_destructive_action_first",
    ):
        if accessibility[key] is not True:
            raise ProductionOperationsContractError(
                f"accessibility invariant {key} changed"
            )
    if (
        accessibility["color_or_icon_only_meaning"] is not False
        or accessibility["watch_events_announced_individually"] is not False
        or accessibility["wcag"] != "2.2"
    ):
        raise ProductionOperationsContractError(
            "accessibility safety changed"
        )

    verification = _exact_keys(
        document["verification"],
        {
            "contract",
            "nonactivation",
            "rules",
            "providers",
            "ui",
            "resources",
            "recovery",
        },
        "verification",
    )
    for key, value in verification.items():
        _unique_strings(value, f"verification.{key}")

    traceability = document["traceability"]
    if not isinstance(traceability, list) or len(traceability) != 8:
        raise ProductionOperationsContractError(
            "traceability must contain PO-R01..PO-R08"
        )
    for index, entry in enumerate(traceability, start=1):
        item = _exact_keys(
            entry, {"id", "requirement", "phase", "owner"}, "traceability entry"
        )
        if (
            item["id"] != f"PO-R{index:02d}"
            or item["phase"] != f"PO{index}"
            or not isinstance(item["requirement"], str)
            or not isinstance(item["owner"], str)
        ):
            raise ProductionOperationsContractError(
                "traceability identity or phase changed"
            )
    _unique_strings(document["external_gates"], "external gates", exact_count=10)


def validate_contract(document: Any, *, check_digest: bool = True) -> dict[str, int]:
    document = _exact_keys(document, EXPECTED_TOP_LEVEL, "PO0 contract")
    if (
        document["schema"] != 1
        or document["phase"] != "PO0"
        or document["status"] != "proposed-not-authorized"
    ):
        raise ProductionOperationsContractError(
            "PO0 contract identity or status changed"
        )
    authority = _exact_keys(
        document["authority"], set(EXPECTED_AUTHORITY), "authority"
    )
    if authority != EXPECTED_AUTHORITY:
        raise ProductionOperationsContractError(
            "PO0 authority gained activation or drifted"
        )
    _validate_ownership(document)
    formats = _exact_keys(document["formats"], set(EXPECTED_FORMATS), "formats")
    if formats != EXPECTED_FORMATS:
        raise ProductionOperationsContractError(
            "PO0 serialization or compatibility changed"
        )
    _validate_evidence(document)
    _validate_record_payloads(document)
    _validate_decisions(document)
    _validate_providers(document)
    _validate_action_profiles(document)
    _validate_planned_surface_and_storage(document)
    _validate_assurance(document)
    if check_digest and canonical_digest(document) != EXPECTED_CANONICAL_SHA256:
        raise ProductionOperationsContractError(
            "reviewed PO0 contract digest changed"
        )
    return {
        "record_kinds": len(document["record_kinds"]),
        "record_payloads": len(document["record_payloads"]),
        "hard_gates": len(document["hard_gates"]),
        "provider_profiles": len(document["provider_profiles"]),
        "action_profiles": len(document["action_profiles"]),
        "actions": sum(
            len(profile) for profile in document["action_profiles"].values()
        ),
        "limits": len(document["limits"]),
        "traceability": len(document["traceability"]),
        "external_gates": len(document["external_gates"]),
    }


def _require_markers(path: Path, markers: tuple[str, ...]) -> None:
    text = _bounded_text(ROOT / path, MAX_DOCUMENT_BYTES, path.as_posix())
    for marker in markers:
        if marker not in text:
            raise ProductionOperationsContractError(
                f"{path.as_posix()} lost required marker {marker!r}"
            )


def _validate_documents() -> None:
    required = {
        CONTRACT_DOC_PATH: (
            "**Status: proposed and non-activating.**",
            "## 2026 architecture decision",
            "### Exact payload catalog",
            "### Freshness profiles",
            "### Kubernetes rollout intent",
            "### Stable lexicographic order",
            "## Provider profiles",
            "### Initial action profile catalog",
            "## Preflight and one-use action lifecycle",
            "## Planned configuration and actions",
            "## Traceability and implementation slices",
            EXPECTED_CANONICAL_SHA256,
        ),
        SPEC_PATH: (
            "## Status and promise",
            "## Architecture and ownership",
            "## Delivery phases",
        ),
        UX_PATH: (
            "## Interaction contract",
            "## Onboarding, settings, and removal",
            "## Accessibility and language",
        ),
        TESTING_PATH: (
            "## Scenario inventory",
            "## Performance, resource, and storage evidence",
            "## Phase exit criteria",
        ),
        ADR_PATH: (
            "Status: Proposed",
            "## Proposed decision",
            "## Acceptance conditions",
        ),
        ROADMAP_PATH: (
            "PO0 is partially done only at a documentation/research-planning boundary",
            "PO1-PO8 are not done",
        ),
        AUDIT_PATH: (
            "PO0 is partially implemented at the detailed proposal/checker boundary",
            "PO1-PO8 are not implemented",
        ),
    }
    for path, markers in required.items():
        _require_markers(path, markers)


def _validate_nonactivation() -> int:
    forbidden_paths = (
        ROOT / "automexia-operations-model",
        ROOT / "automexia-devops/src/operations",
        ROOT / "apps/automexia-terminal/src/automexia/operations",
        ROOT / "apps/automexia-terminal/src/screen/operations.rs",
        ROOT / "apps/automexia-terminal/src/renderer/operations.rs",
    )
    for path in forbidden_paths:
        if path.exists():
            raise ProductionOperationsContractError(
                f"PO0 unexpectedly gained runtime path {path.relative_to(ROOT).as_posix()}"
            )

    source_roots = (
        ROOT / "apps/automexia-terminal/src",
        ROOT / "automexia-devops/src",
        ROOT / "automexia-ui-model/src",
        ROOT / "rio-backend/src",
    )
    forbidden_source_markers = (
        "RefreshProductionContext",
        "OpenProductionSituation",
        "ReviewOperationalCandidate",
        "ToggleIncidentMode",
        "StopManagedOperation",
        "production_operations",
        "ProductionOperationsConfig",
    )
    checked = 0
    for source_root in source_roots:
        for path in source_root.rglob("*.rs"):
            checked += 1
            text = _bounded_text(
                path,
                MAX_DOCUMENT_BYTES,
                path.relative_to(ROOT).as_posix(),
            )
            for marker in forbidden_source_markers:
                if marker in text:
                    raise ProductionOperationsContractError(
                        f"{path.relative_to(ROOT).as_posix()} gained activating marker {marker!r}"
                    )

    cargo = _bounded_text(ROOT / "Cargo.toml", MAX_DOCUMENT_BYTES, "Cargo.toml")
    if re.search(r"automexia[-_]operations[-_]model", cargo):
        raise ProductionOperationsContractError(
            "workspace unexpectedly gained the PO model crate"
        )
    for config_path in (
        Path("docs/CONFIGURATION.md"),
        Path("docs/reference/configuration.md"),
    ):
        text = _bounded_text(
            ROOT / config_path, MAX_DOCUMENT_BYTES, config_path.as_posix()
        )
        if "[production-operations]" in text:
            raise ProductionOperationsContractError(
                f"{config_path.as_posix()} claims unimplemented PO settings"
            )
    return checked


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    if root != ROOT:
        raise ProductionOperationsContractError(
            "alternate repository roots are not supported"
        )
    text = _bounded_text(
        ROOT / CONTRACT_PATH, MAX_CONTRACT_BYTES, "PO0 contract"
    )
    document = parse_contract(text)
    counts = validate_contract(document)
    _validate_documents()
    counts["source_files_checked"] = _validate_nonactivation()
    return counts


def main() -> int:
    try:
        counts = validate_repository()
    except (OSError, UnicodeError, ProductionOperationsContractError) as error:
        print(f"Production Operations PO0 validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: Production Operations PO0 remains strict, detailed, and non-activating "
        f"(records={counts['record_kinds']}, payloads={counts['record_payloads']}, "
        f"gates={counts['hard_gates']}, profiles={counts['provider_profiles']}, "
        f"actions={counts['actions']}, limits={counts['limits']}, "
        f"traceability={counts['traceability']}, source-files={counts['source_files_checked']}, "
        f"external={counts['external_gates']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
