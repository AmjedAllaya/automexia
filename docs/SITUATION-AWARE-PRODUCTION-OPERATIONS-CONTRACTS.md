# Production Operations PO0 contracts and 2026 implementation decisions

## Status, authority, and scope

**Status: proposed and non-activating.** This page freezes the detailed PO0
design candidate for review. It does not accept
[ADR 0034](adr/0034-situation-aware-production-operations.md), add a runtime
dependency, create a setting or shortcut, contact a provider, start a watcher,
publish a completion, persist a journal, or authorize execution. PO1-PO8 remain
unimplemented.

The canonical product behavior is in
[Situation-Aware Production Operations](SITUATION-AWARE-PRODUCTION-OPERATIONS.md);
the [UX blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md) owns visible
surfaces and journeys; the
[testing contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md) owns future
evidence. This page owns the exact proposed data, rule, adapter, policy,
lifecycle, configuration, migration, and dependency decisions needed before
implementation.

The machine-readable mirror is
`tests/fixtures/production-operations/po0-contract-v1.json`. Its checker and
mutation suite enforce nonactivation and prevent silent weakening. The JSON
record is a reviewed planning contract, not a runtime configuration file. Its
current RFC 8785-style canonical JSON SHA-256 is
`8038ce24aa0f4223a910ed2293f9c40b17882e617fda9a01a14d13f65d7004cf`;
any change requires renewed review and a deliberate checker update.

## Outcome and acceptance criteria

The feature should help an operator answer five questions with less searching
and fewer context mistakes:

1. Where am I operating, as which identity, and how fresh is that information?
2. What is observed, what is inferred, what conflicts, and what remains unknown?
3. Which read-only investigation or reviewed native command is most relevant?
4. What could the action affect, and what external authority permits or blocks it?
5. If a managed action is separately enabled later, did it stabilize and verify,
   and what recovery remains available?

PO0 is complete only after the ADR and exact machine-contract digest are accepted
by the project owners, nonactive prototypes are reviewed, dependency and
capability owners are named, checker mutations pass, and the released runtime is
proven to contain no PO capability, provider request, watcher, setting, shortcut,
surface, journal, or action. Documentation alone remains partial evidence.

## 2026 architecture decision

### Build, wrap, and adopt

| Concern | Decision | Why |
|---|---|---|
| Evidence, risk, rule, policy, ranking, and lifecycle model | **Build** a small private pure Rust model | These contracts express Automexia's product and safety policy. The model must have no renderer, PTY, shell, filesystem, process, network, provider SDK, credential, persistence, or model-runtime authority. |
| Editor insertion and candidate surface | **Wrap** CP5 and preserve CP1 | CP5 already owns authenticated replacement-only insertion and CP1 is the immediate fallback. A second completion popup or shell bridge would create conflicting ownership. |
| Context, credentials, and provider CLI plans | **Wrap** D2 and existing D6/M7-M12 first-party adapters | They already use explicit profiles, opaque credential references, bounded public parsing, typed executables, exact argument arrays, and nonactivation. |
| External execution and child processes | **Wrap** the existing disabled D3/application broker | It already owns exact executable/argv, one process tree, cancellation, receipts, and cleanup. PO must never create a second runner. |
| Kubernetes one-shot reads and familiar commands | **Wrap** reviewed `kubectl`/`oc` argv through the existing adapter | It minimizes new dependencies, matches the user's selected context, and is appropriate for explicit inspection and inserted commands. Structured JSON is parsed; terminal-formatted tables are not an authority. |
| Kubernetes sustained list/watch | **Conditionally adopt** `kube`/`kube-runtime` in the optional first-party adapter after a separate dependency review | In 2026 `kube-rs` is a CNCF Sandbox project with automatic relist-aware watchers. A maintained client is safer than reimplementing watch state, but its TLS, proxy, exec-plugin, binary-size, MSRV, transitive, and resource effects must pass the repository checklist. It is not adopted by PO0. |
| Cloud inventory and audit reads | **Wrap** exact vendor CLI/API adapters per provider | Do not add all vendor SDKs to the terminal. Start with bounded explicit reads; adopt an SDK only when typed API behavior, pagination, cancellation, or sustained polling cannot meet the contract through the existing adapter. |
| GitOps, policy, and observability semantics | **Adopt external authority, normalize its result** | Argo CD, Flux, OPA, cloud IAM, Kubernetes admission, SLO systems, and provider audit logs retain their own semantics. Automexia must not recreate them. |
| Journals and packs | **Wrap** private atomic storage and the D7 signed-package boundary | No SQLite database or second package trust system is justified for bounded content-free receipts and declarative packs. |
| Initial intelligence | **Build deterministic rules only** | A restart recommendation must be explainable from typed facts. A small local reorder-only model is a PO8 experiment, never a source of commands, targets, policy, or execution. |

No PO0 production dependency is added. A later `kube-rs` review must record the
exact pinned versions and features, `k8s-openapi` compatibility, one TLS stack,
license and advisory state, MSRV, build and binary size, cold and steady memory,
watch recovery, proxy and exec-plugin behavior, cancellation, disable/uninstall,
and native Windows/Linux/macOS evidence. Default features must not be accepted
without review.

### Process and trust boundaries

```text
explicit request or parked bounded watcher event
  -> first-party adapter validates public provider data
  -> pure neutral evidence records
  -> route-scoped immutable evidence snapshot
  -> deterministic domain rules
  -> hard safety and external-authority gates
  -> stable lexicographic ranking
  -> immutable UI projection through CP5/detail/preflight
  -> insertion without Enter, or separately activated one-use managed grant
  -> existing broker, observation, stabilization, verification, receipt
```

Provider output, Kubernetes objects, log summaries, labels, annotations, paths,
GitOps metadata, policy text, command completion text, and organization packs are
untrusted input. Display text never becomes an executable, argument, identifier,
selector, or capability. All publication is route-, environment-revision-, and
generation-bound; a later generation cancels and invalidates the earlier one.

## Contract format and compatibility

### Representation rules

| Use | Required representation |
|---|---|
| In-process domain records | Rust enums and structs with explicit bounded wrappers; no `serde_json::Value` after ingress |
| Public pack, fixture, export, or adapter interchange | Strict UTF-8 JSON validated against JSON Schema 2020-12 |
| User configuration | Existing `config.toml` owner and kebab-case conventions |
| Time | RFC 3339 UTC plus the original source timestamp and clock-quality state when available |
| Version | SemVer 2.0.0 for public record/rule/profile versions; an incompatible field or meaning changes the major version |
| Digest | RFC 8785 JSON Canonicalization Scheme followed by SHA-256 for public canonical records |

Ingress rejects duplicate keys before schema validation, trailing data, invalid
UTF-8, non-finite numbers, overlong strings, excess depth/count/bytes, control
or bidi characters in identifiers, and unknown fields unless that exact schema
declares a bounded extension map. JSON Schema `format` annotations are not the
only validation: timestamps, identifiers, byte ceilings, numeric ranges, and
cross-field invariants are checked in code.

Minor-version readers may accept an older minor record of the same major only
when required semantics are unchanged. They do not silently ignore a new action,
permission, risk, policy, or redaction field. Unknown major versions fail closed.
No runtime migration reads the planning fixture.

PO0 deliberately does not create a runtime JSON Schema file. Before a phase can
publish or persist one of these records, that phase must add its versioned
JSON Schema 2020-12 document, strict parser, semantic validator, valid/invalid
fixtures, compatibility tests and byte/depth/count limits. The planning mirror
defines the reviewed fields; it is not accepted as a substitute for those
runtime artifacts.

### Common envelope

Every external observation and durable content-free receipt uses these fields:

| Field | Type and limit | Meaning |
|---|---|---|
| `schema-version` | SemVer, 32 bytes | Record contract, not provider version |
| `kind` | registered enum, 64 bytes | Exact record kind |
| `record-id` | opaque 128-bit/random or source-stable ID, 128 bytes | Never derived from secret content |
| `route-id` | opaque 128 bytes | Pane/session route owner |
| `environment-revision` | nonzero integer | Context generation owned by D2 |
| `generation` | nonzero integer | Request/publication generation |
| `source-id` | registered adapter/profile ID, 128 bytes | Provenance owner |
| `source-version` | SemVer or exact vendor version, 64 bytes | Parser/adapter compatibility |
| `observed-at` | RFC 3339 UTC | Provider's event time when it supplies one |
| `received-at` | RFC 3339 UTC | Automexia ingress time |
| `expires-at` | RFC 3339 UTC or explicit `unknown` state | Eligibility horizon, not a cache hint |
| `source-sequence` | bounded source token or absent | Watch resourceVersion, cursor, event ID, or sequence |
| `coverage` | `complete`, `partial`, `sampled`, `gapped`, `unknown` | What the source actually covered |
| `clock-quality` | `trusted`, `source`, `ingress-only`, `skewed`, `unknown` | Whether ordering by time is defensible |
| `redaction-class` | `public-metadata`, `content-minimized`, `private-reference` | Storage/export rule |
| `canonical-sha256` | 64 lowercase hex characters | Digest of the canonical record excluding this field |

`observed-at` can be absent only when `clock-quality` is `ingress-only` or
`unknown`. `expires-at=unknown` makes production mutation ineligible. Source
sequence values are opaque: they may be compared only by the owning adapter.

## Exact neutral records

### Environment passport

`EnvironmentPassport` contains:

- route, environment revision, lock generation, environment class and the
  provenance of that classification;
- provider and explicit profile/configuration reference;
- account, subscription, project or tenant public identifier;
- region/zone, cluster server-origin fingerprint, cluster UID when available,
  namespace/project, and selected resource scope;
- effective identity public fingerprint, groups/role summary, authentication
  expiry state, and JIT/elevation state;
- GitOps controller/application/revision and reconciliation state;
- incident reference and change-window state;
- one `PassportField` per value with source, observation time, expiry, quality,
  and redaction class; and
- the canonical digest used by candidates and preflight.

The passport never stores access keys, tokens, certificates, kubeconfig bodies,
exec-plugin output, client secrets, refresh tokens, environment values, or raw
credential-cache paths. A path or credential may appear only as an opaque
private reference owned by the existing provider boundary.

Environment classification is one of `production`, `staging`, `development`,
`sandbox`, or `unknown`. Precedence is:

1. signed organization mapping of stable account/cluster/project identity;
2. reviewed local exact-identity mapping;
3. provider-native explicit metadata with an approved key and value;
4. heuristic name matching, shown only as `inferred` and never sufficient to
   classify a mutation target as non-production;
5. otherwise `unknown`.

`unknown` receives production safety treatment for mutation.

### Evidence and knowledge

An `Observation` contains a registered semantic subject, predicate, typed value,
source reference, scope, time window, coverage, and quality. It never contains
raw YAML, an unbounded provider body, a log line, terminal output, or a metric
series.

The knowledge state is exactly one of:

- `observed`: directly returned by the named current authority;
- `derived`: deterministic calculation from linked observations;
- `correlated`: temporally or topologically related but not proof of cause;
- `policy`: decision from a named policy authority and exact revision;
- `conflicting`: two eligible sources disagree;
- `unknown`: required evidence is missing, stale, unauthorized, unsupported, or
  outside source coverage.

Every finding has supporting, contradicting, and missing evidence references.
No numeric confidence is displayed as probability unless a separately validated
statistical model defines and calibrates it. The initial deterministic system
uses quality classes, not an opaque “AI confidence” score.

### Exact payload catalog

Every record also carries the common envelope where applicable. The payload
field names below are exact: removing, renaming or adding one requires a
versioned contract change. The machine mirror and mutation checker enforce all
17 sets, not only the security-sensitive examples.

| Record | Exact payload fields |
|---|---|
| `environment-passport` | `environment-class`, `classification-source`, `provider`, `profile-reference`, `account-subscription-project-tenant`, `region-zone`, `cluster-server-origin-fingerprint`, `cluster-uid`, `namespace-project-scope`, `effective-identity-fingerprint`, `identity-groups-role-summary`, `authentication-expiry-state`, `jit-elevation-state`, `gitops-owner-revision-state`, `incident-reference`, `change-window-state`, `passport-fields`, `lock-generation`, `passport-digest` |
| `observation` | `subject-reference`, `predicate-id`, `typed-value`, `value-schema`, `scope`, `time-window`, `quality`, `provider-native-reference` |
| `evidence-snapshot` | `passport-digest`, `observation-references`, `resource-node-references`, `resource-edge-references`, `supporting-evidence`, `contradicting-evidence`, `missing-sources`, `snapshot-quality`, `snapshot-digest` |
| `change-record` | `resource-reference`, `change-kind`, `actor-public-reference`, `provider-event-reference`, `changed-semantic-fields`, `before-digest`, `after-digest`, `resolved-revision`, `correlation-window`, `causal-authority` |
| `field-ownership-record` | `resource-reference`, `semantic-field-id`, `manager-public-reference`, `owner-kind`, `operation-kind`, `desired-source-reference`, `last-change-reference`, `ownership-quality` |
| `resource-explanation` | `resource-reference`, `state-class`, `reason-code`, `current-semantic-state`, `desired-semantic-state`, `blockers`, `owner-chain`, `dependency-references`, `supporting-evidence`, `contradictions`, `unknowns` |
| `comparison-finding` | `subject-reference`, `baseline-kind`, `cohort-definition`, `baseline-references`, `normalized-field-ids`, `equal-fields`, `different-fields`, `unknown-fields`, `revision-mapping`, `coverage`, `finding-state` |
| `network-path-assessment` | `origin-vantage`, `destination-reference`, `protocol-port`, `ordered-layers`, `layer-states`, `evidence-references`, `untested-layers`, `active-probe-used`, `path-conclusion` |
| `slo-impact-summary` | `service-reference`, `objective-reference`, `evaluation-window`, `recording-rule-reference`, `burn-rate-windows`, `request-volume`, `missing-evaluations`, `low-traffic-state`, `impact-class`, `root-cause-authority` |
| `situation-assessment` | `operational-intent`, `subject-references`, `finding-references`, `hypotheses`, `supporting-evidence`, `contradictions`, `missing-evidence`, `eligible-rule-ids`, `assessment-state` |
| `candidate-action` | `action-id`, `tool-profile-id`, `effect-class`, `typed-operation`, `target-uids`, `safety-class`, `reason-message-id`, `evidence-references`, `hard-gate-results`, `refusal-reasons`, `verification-profile-id`, `recovery-profile-id`, `ranking-key`, `candidate-digest` |
| `policy-decision` | `decision-state`, `policy-source`, `policy-revision`, `decision-id`, `subject-digest`, `action-digest`, `target-digest`, `environment-digest`, `obligations`, `decision-expiry`, `decision-coverage` |
| `impact-preflight` | `passport-digest`, `candidate-digest`, `evidence-snapshot-digest`, `exact-executable-or-api-operation`, `exact-arguments-or-typed-request`, `target-uids`, `target-count`, `dependency-impact`, `authorization-result`, `admission-limitations`, `gitops-state`, `policy-decisions`, `jit-credential-and-approval-state`, `change-window-state`, `verification-plan`, `stabilization-window`, `timeout`, `recovery-plan`, `preflight-digest` |
| `managed-action-grant` | `operation-id`, `route-passport-evidence-policy-preflight-digests`, `exact-executable-or-api-operation`, `exact-arguments-or-typed-request`, `environment-allowlist`, `target-uids`, `capability-principal`, `authorization-reference`, `target-count-ceiling`, `timeout`, `verification-profile-id`, `recovery-profile-id`, `broker-generation`, `expires-at`, `consumption-state` |
| `operation-monitor` | `operation-id`, `operation-state`, `exact-target-summary`, `started-at`, `elapsed-time`, `deadline`, `before-state-reference`, `last-observation-reference`, `stabilization-state`, `verification-state`, `cleanup-state`, `cancel-stop-availability`, `failure-or-uncertainty-reason`, `recovery-reference` |
| `managed-diagnostic-session` | `session-id`, `session-kind`, `operation-id`, `target-uids`, `endpoint-or-vantage`, `protocol-port`, `immutable-image-or-profile`, `authorization-and-admission-references`, `traffic-and-time-limits`, `listener-process-and-temporary-resource-owners`, `detach-policy`, `cleanup-plan`, `cleanup-verification` |
| `action-receipt` | `operation-id`, `route-and-passport-digest`, `action-and-rule-ids`, `target-public-identity-digest`, `environment-and-risk-class`, `provider-and-profile-versions`, `preflight-policy-and-grant-digests`, `public-decision-ids`, `start-and-end-time`, `final-state`, `cancellation-or-timeout-reason`, `verification-and-recovery-profile-ids`, `verification-and-recovery-outcomes`, `cleanup-ownership-references` |

Collection adapters may temporarily hold bounded provider-native data while
validating it, but the pure model, cache, journal and UI receive only these
neutral fields. A record reference is resolved only by its owning adapter or
bounded snapshot; it is never interpreted as a path, URL, selector or command.

### Evidence quality

Quality is the tuple:

```text
authority × provenance × freshness × coverage × directness × clock × gaps
```

Each dimension is explicit. A finding is `eligible`, `degraded`, or `ineligible`
for a named use; there is no average score that lets strong freshness hide
missing authorization or coverage.

| Condition | Read-only display | Ranking | Production mutation |
|---|---|---|---|
| Fresh, direct, complete | Eligible | Eligible | Eligible only with all independent gates |
| Fresh but partial/sampled | Show coverage | Eligible for diagnostics with penalty | Ineligible if omitted data could change target, impact, permission, policy, or verification |
| Stale | Show last-known-good and age | Diagnostic-only with penalty | Ineligible |
| Conflicting | Show both sources and resolution hint | Prefer evidence-gathering action | Ineligible |
| Unauthorized/unavailable | Show missing source without leaking policy details | Do not invent fact | Ineligible |
| Clock unknown or gapped | Show source-local ordering only | Do not claim global chronology | Ineligible when time/window is a gate |

### Freshness profiles

These are maximum ages, not polling intervals. Provider-specific lower limits
win. A user may tighten but not raise a safety ceiling.

| Evidence class | Normal read-only | Production read-only | Mutation preflight |
|---|---:|---:|---:|
| Route, account/project, cluster, namespace, effective identity | 60 s | 15 s | refreshed in the preflight epoch |
| Target existence, UID, generation, owner chain, rollout/health | 15 s | 5 s | 5 s and rechecked after confirmation |
| Exact authorization/IAM capability | 30 s | 10 s | current exact-action check; completed within 5 s of grant |
| Policy, change window, JIT/elevation and credential expiry | 30 s | 10 s | refreshed in the preflight epoch |
| GitOps owner, desired revision, sync window and reconciliation | 30 s | 15 s | refreshed in the preflight epoch |
| Kubernetes watch data | 15 s since a meaningful event/relist | 5 s | target GET/list plus resourceVersion/UID check |
| Deployment/change/audit event | source retention plus 60 s collection age | same | advisory; never the sole authorization gate |
| SLO/recording-rule result | two evaluation intervals, maximum 120 s | two intervals, maximum 60 s | impact evidence only; never authority |
| Log summary | 30 s | 10 s | diagnostic/verification evidence only |
| Organization metadata | declared expiry, maximum 24 h | declared expiry, maximum 1 h | exact current revision required |

A Kubernetes watch bookmark proves progress in a watch stream; it does not by
itself refresh object health. `410 Gone`, disconnect, missed sequence, relist,
throttle, or authorization loss changes coverage and freshness visibly.

## Deterministic rule system

### Rule profile

Each built-in or organization-contributed rule declares:

| Field | Contract |
|---|---|
| Identity | `rule-id`, SemVer, publisher/built-in owner, domain, supported tool/profile versions |
| Trigger | Typed intent plus predicates over neutral observations; no regex over raw logs as an execution trigger |
| Required evidence | Exact record kinds, scopes, maximum ages, minimum coverage, and authority |
| Supporting/contradicting evidence | Named predicates and explicit unknown result |
| Candidate templates | Registered action IDs and typed parameter mappings only |
| Hard suppressors | Cause, policy, context, target, version, permission, GitOps, impact, or verification conditions that refuse the candidate |
| Explanation | Fixed message key with bounded public substitutions; never provider text interpreted as a command |
| Verification and recovery | Registered predicate/profile IDs |
| Tests | Positive, negative, boundary, stale, conflicting, unsupported-version, and no-side-effect fixture IDs |

Rules are pure, deterministic, side-effect-free, order-independent, and bounded.
An organization pack may add service criticality, ownership, approved evidence
sources, declarative predicates, candidate restrictions, verification profiles,
and one-step runbook links. It cannot add code, script text, shell strings,
callbacks, network locations, credentials, capabilities, or a new action kind.

### Kubernetes rollout intent

When the editor contains `kubectl rollout`, PO first preserves native syntax
completion. Situation candidates are added only after the context, intent,
affected controller UID, current health, owner chain, rollout state, change and
GitOps state are known.

| Observed situation | First situation candidate | Restart treatment |
|---|---|---|
| New Deployment revision is progressing normally | `kubectl rollout status deployment/<name> --namespace <ns>` | Suppressed; no failure exists |
| Progress deadline exceeded after a new revision | Status, history, new/old ReplicaSet comparison, then a reviewed `undo` candidate when the previous revision is known healthy and policy permits | Restart ranks below diagnose/undo because it preserves the broken revision |
| Pods are `ImagePullBackOff` or `ErrImagePull` | Inspect image reference, pull events, registry/identity policy, and the owning controller | Refused as a fix; restart does not correct image or credentials |
| Pods are Pending/Unschedulable | Explain requests, node capacity, affinity, taints/tolerations, PVC and scheduling gates | Refused as a fix; do not simulate the scheduler |
| One replica fails readiness while healthy peers on the same revision exist | Compare the failed Pod with a healthy peer, events, node, config references and endpoint membership | May appear as low-priority reviewed recreation only if the owner can safely replace one replica; broad rollout restart is suppressed |
| All replicas fail readiness after a correlated config/secret/dependency change | Inspect the exact change and dependency; compare previous revision/config; prefer rollback or dependency recovery when available | Restart is suppressed unless evidence proves a transient external condition has recovered and policy permits |
| `CrashLoopBackOff` with stable configuration | Inspect termination reason/exit, previous logs summary, events, probes, dependency and OOM/resource evidence | Not assumed to be root cause; a restart is at most a reversible test with explicit uncertainty |
| OOMKilled or repeated resource-pressure eviction | Explain request/limit/node pressure and compare healthy cohort | Restart is temporary and ranks below a reviewed resource/deployment correction |
| Service has no ready endpoints | Check selector, EndpointSlice readiness/serving/terminating, Pod readiness and owner health | Restart only if a specific eligible owner/cause supports it; never because the Service is empty alone |
| Node/network/DNS path is degraded | Diagnose the named layer and vantage point | Workload restart is suppressed unless independent evidence isolates workload state |
| GitOps owns the workload and will self-heal/revert direct changes | Show desired/live difference, controller, revision and sync window | Direct action is blocked or labelled temporary according to policy; prefer GitOps-native change |
| No stable owner UID, evidence is stale/conflicting, or authorization is unknown | Refresh/investigate/refusal row | No mutation candidate |

The ranked result never says “this deployment needs a restart.” It says, for
example, “Restart is a reviewed option because all Pods on revision X remain
unready after dependency Y recovered; evidence is 3 s old; GitOps will reconcile
this field.” Business criticality changes impact and review order only when a
reviewed organization source supplies it.

### Cross-tool situation families

The first supported families are deliberately narrow:

| Family | Inputs | Safe initial output |
|---|---|---|
| Kubernetes rollout/workload health | Typed API state, owner UIDs, events, revisions, EndpointSlices, permissions, GitOps | Read-only diagnosis, status/history/undo/restart candidate with refusal |
| Cloud identity/context | Explicit AWS profile, Azure subscription/tenant, GCP configuration/project, public caller identity and expiry | Passport, refresh, provider-owned login/elevation link |
| Cloud deployment change | Bounded audit/change record plus deployment-native status/preview | “What changed?”, correlation, native diff/what-if/plan limitations |
| GitOps drift/reconciliation | Argo/Flux application, revision, conditions, inventory, ignore/self-heal/sync-window state | Desired/live explanation and GitOps-native next step |
| IaC preview | Sanitized summary produced by an approved Terraform/OpenTofu/Helm adapter | Counts and redacted references; never raw plan/state persistence |
| Network path | Passive DNS/Service/EndpointSlice/backend/cloud metadata plus explicit vantage | Layer-by-layer known/unknown result; active probe deferred to PO6 |
| User impact | Approved SLO/recording-rule result and request-volume window | Burn/impact summary with low-traffic and coverage limits; never root-cause proof |

## Candidate gates and ranking

### Hard gates

A candidate is refused before ranking when any applicable condition is true:

1. route, environment revision, passport digest, target UID or generation changed;
2. required evidence is stale, conflicting, missing, gapped beyond the rule, or
   from an unsupported adapter/tool version;
3. environment classification is unknown and the action is treated as
   non-production;
4. target count or blast-radius limit is exceeded;
5. exact permission, external admission prerequisite, policy, GitOps rule,
   change window, JIT state, approval or credential horizon is denied/unknown;
6. exact typed action, verification, timeout or recovery profile is unregistered;
7. the action relies on raw/untrusted text, shell evaluation, implicit Enter,
   a global CLI context mutation, or an uncontrolled external diff program;
8. the action cannot be bound to stable provider identity and resource UID; or
9. the operation owner, cancellation, cleanup or receipt path is unavailable.

Read-only diagnostics may remain available when a mutation is refused. The row
must name the blocker and a safe way to refresh or investigate.

### Stable lexicographic order

Eligible candidates use this ordered tuple; later fields never override an
earlier one:

```text
1 situation match: exact affected resource > dependency/owner > broad context
2 operator intent: requested verb/noun match > adjacent diagnostic > unrelated
3 safety class: read-only > reversible reviewed mutation > irreversible/broad
4 evidence quality: eligible direct > eligible derived > degraded diagnostic
5 causal usefulness: tests/discriminates hypothesis > observes > masks symptom
6 blast radius: one resource > one workload > namespace > cluster/account
7 verification quality: exact current predicate > bounded proxy > unavailable
8 recovery quality: known rehearsed > known unproven > unavailable
9 organization priority: reviewed service metadata only
10 deterministic tie: action ID, target canonical identity, rule ID
```

Risk never becomes a hidden weighted score. The UI shows the decisive reasons:
for example `matches failed rollout`, `read-only`, `evidence 4 s old`, `one
Deployment`, and `verification available`.

## Policy and external authority

### Normalized policy decision

`PolicyDecision` contains `allow`, `deny`, or `unknown`; policy source and
revision; decision ID; evaluated subject/action/target/environment digests;
obligations; expiry; coverage; and provenance. Input and result bodies are not
stored. OPA `undefined`, provider error, timeout, expired data, malformed
obligation, or missing policy becomes `unknown`.

Precedence is:

1. Automexia hard safety boundary;
2. live provider/Kubernetes authorization and admission prerequisites;
3. organization deny and mandatory obligations;
4. environment/account/cluster policy;
5. signed runbook-pack restrictions;
6. user preference.

Any deny wins. `unknown` blocks production mutation. A lower level cannot
override a higher deny or remove an obligation. Conflicting allows are not
combined into broader authority. Exceptions happen only through the external
approved system; Automexia refreshes and records the resulting decision.

Authorization is not admission and simulation is not execution. A Kubernetes
`SelfSubjectAccessReview`, cloud IAM simulation, ARM what-if, Terraform plan,
Helm dry run, or GitOps diff may still be followed by an admission, quota,
concurrency, provider, or runtime failure. The review says exactly which
authority was and was not consulted.

## Provider profiles

Each profile declares compatible tool/API versions, exact public evidence,
explicit scope/profile arguments, credential reference type, pagination/watch
algorithm, quotas, freshness, redaction, unsupported states, authorized action
IDs, cleanup, and native fixtures. Unknown versions disable semantic parsing and
fall back to native completion or a plain reviewed command.

### Initial action profile catalog

These 37 IDs are the exact initial PO0 vocabulary. `phase` means the earliest
delivery phase allowed to expose the candidate; it does not claim the phase is
implemented. `reviewed-mutation` in PO4 means review and insertion only. Managed
execution still requires PO6, a fresh one-use grant and the existing broker.

| Profile | Exact planned actions |
|---|---|
| Kubernetes | `kubernetes.context.refresh` — PO1/read-only; `kubernetes.workload.explain`, `kubernetes.resource.compare-healthy`, `kubernetes.network.explain` — PO2/read-only; `kubernetes.rollout.status`, `kubernetes.rollout.history` — PO3/read-only; `kubernetes.rollout.undo`, `kubernetes.rollout.restart` — PO4/reviewed-mutation; `kubernetes.port-forward`, `kubernetes.debug-workload` — PO6/managed-session |
| OpenShift | `openshift.context.refresh` — PO1/read-only; `openshift.workload.explain`, `openshift.resource.compare-healthy` — PO2/read-only; `openshift.rollout.status` — PO3/read-only |
| AWS | `aws.identity.refresh` — PO1/read-only; `aws.changes.lookup` — PO2/read-only; `aws.permission.review` — PO4/advisory |
| Azure | `azure.identity.refresh` — PO1/read-only; `azure.changes.lookup` — PO2/read-only; `azure.permission.review` — PO4/advisory; `azure.deployment.what-if` — PO4/simulation |
| Google Cloud | `gcp.identity.refresh` — PO1/read-only; `gcp.changes.lookup`, `gcp.asset-history.lookup` — PO2/read-only; `gcp.permission.review` — PO4/advisory |
| GitOps and IaC | `gitops.desired-live.diff`, `gitops.reconciliation.status`, `helm.release.status`, `terraform.plan-summary.inspect` — PO2/read-only; `gitops.sync-window.review` — PO4/advisory; `helm.release.simulate-upgrade` — PO4/simulation; `gitops.reconcile` — PO6/managed-mutation |
| Policy and observability | `observability.slo-impact`, `observability.log-summary` — PO2/read-only; `observability.trace-correlate` — PO2/correlation-only; `policy.decision.refresh`, `identity.elevation.status` — PO4/read-only |

Each machine entry also pins its evidence authority. No profile may translate a
display string into an operation. Adding an ID requires provider research,
typed request/argv ownership, effect classification, phase ownership,
authorization/admission limits, refusal rules, verification/recovery profiles,
resource ceilings, fixtures and a contract-version review.

OpenShift initially receives no OpenShift-specific mutation. Standard
Kubernetes resource actions may reuse the Kubernetes action contract only after
the exact API kind, server/client compatibility, authorization, GitOps owner,
verification and recovery behavior pass that profile. DeploymentConfig or other
OpenShift-specific mutations require their own researched action IDs.

### Kubernetes and OpenShift profile

- Bind every request to the reviewed kubeconfig source digest, context, server
  origin, namespace/project, identity, cluster fingerprint and route revision.
- Use `SelfSubjectReview` for effective identity where supported and exact
  `SelfSubjectAccessReview` per action. Do not treat broad rule enumeration as
  permission for a concrete action; admission may still deny.
- Resolve controllers through `ownerReferences` name **and UID**. Reject invalid
  cross-namespace ownership. Labels are hints, not ownership proof.
- Use an initial bounded list and then namespace/resource-scoped watches.
  Recover `410 Gone` and desynchronization by clearing the affected generation
  and relisting. Atomically publish a new snapshot; never merge half a relist
  with the old generation.
- Use Deployment conditions together with observed generation, desired/updated/
  ready/available/unavailable replicas and reason. `Progressing=True` alone does
  not mean an active healthy rollout.
- Treat Events as best-effort, limited-retention supporting evidence. Treat
  `CrashLoopBackOff` as a symptom and never reproduce the scheduler as a local
  source of truth.
- EndpointSlice readiness, serving and terminating states are evaluated across
  all selected slices and ports. NetworkPolicy configuration is not proof of
  end-to-end connectivity.
- `kubectl` is supported only within the Kubernetes documented one-minor
  version-skew window and the exact tested matrix. `oc` must match the reviewed
  OpenShift client/server compatibility matrix. Unknown versions retain plain
  native completion.
- Never inherit `KUBECTL_EXTERNAL_DIFF`, enable `--show-secrets`, write current
  context, or persist exec-plugin output. Credential expiry unknown is displayed;
  production preflight performs a current authentication probe.
- Port-forward is PO6-only, TCP-only, loopback by default, explicit local port,
  exact target UID, `get pods` plus `create pods/portforward` authorization,
  visible network-control bypass warning, owned listener/process, and stop/
  cleanup receipt.
- Ephemeral debug containers cannot be removed. A debug profile therefore
  requires an immutable approved image, target/container, security context,
  admission result, traffic/data policy and explicit residual-resource wording.
  A copied Pod has a separately owned delete-and-verify cleanup path.

### AWS

- Require an explicit named profile/reference on every CLI/API request; never
  select or rewrite ambient/default configuration.
- Use STS `GetCallerIdentity` for the public caller identity and bind account,
  ARN and profile revision. Keep credential material in AWS/provider custody.
- CloudTrail Event history is optional change evidence only: it is regional,
  account-scoped, management-event-only, limited to 90 days, and `LookupEvents`
  is limited to two requests per second per account/region. The adapter default
  is at most one request per second with bounded pagination and cancellation.
- IAM simulation is advisory and may differ from live authorization, resource
  policy, Organizations controls, VPC endpoint policy, cross-account context or
  service behavior. The final service call remains authoritative.
- IAM Identity Center/PAM elevation remains provider-owned. Automexia shows the
  active public role and expiry, links to the reviewed provider flow, and
  refreshes afterward; it does not obtain or persist temporary credentials.

### Azure

- Every read names the exact tenant/subscription where the CLI supports it.
  Never run `az account set` or mutate shared CLI configuration.
- `az account show` supplies bounded public identity/context; tokens and account
  cache bodies remain external.
- Azure Activity Log is optional change evidence with explicit subscription,
  resource and time window; default platform retention is 90 days unless an
  organization exports it.
- ARM what-if is a preview, not a guarantee. Report unresolved/ignored coverage,
  the 500 nested-template, 800 resource-group and five-minute expansion limits,
  `reference()` false-change limitations, and the fact that deployment-level
  permissions are still required.
- Effective RBAC includes assignments, conditions and deny assignments; one
  displayed role is not sufficient. Entra PIM activation/approval/MFA remains
  provider-owned and is refreshed as external state.

### Google Cloud

- Pass an explicit `--configuration` and project/resource on each CLI request;
  never activate or rewrite the user's global configuration.
- `testIamPermissions` is an exact-resource capability observation, not a
  guarantee that a later operation will pass service policy, quota or runtime.
- Cloud Audit Logs coverage depends on log type and enablement; retention varies
  by bucket. Store bounded summaries and stable references, not log bodies.
- Cloud Asset Inventory history is optional, independently authorized, limited
  to supported asset types and at most 35 days. Missing history is `unknown`,
  not “no change.”
- Privileged Access Manager remains the owner of just-in-time elevation and
  approval.

### Argo CD, Flux, Helm, Terraform, and OpenTofu

- Argo CD and Flux are the desired/live and reconciliation authorities. Preserve
  exact resolved Git revision, application/Kustomization/HelmRelease identity,
  Ready/Reconciling/Stalled state, inventory, ignore rules, field-manager rules,
  self-heal and sync-window decision.
- Multiple Argo sync windows can apply. A Git branch or tag is not an immutable
  revision until resolved to a commit.
- Flux reconciliation and Argo sync are mutations; never call them during a
  read-only refresh.
- Helm `--dry-run=server`/`client` is labelled simulation and always hides
  Secrets. `--atomic`, upgrade, rollback and reconciliation remain managed
  mutations.
- Terraform/OpenTofu JSON plan formats are versioned. Saved plans and
  `show -json` can expose sensitive values in plaintext. PO accepts only an
  approved content-minimized summary containing counts, typed resource
  references, unknown/sensitive masks and the plan digest; raw plan/state is
  neither cached nor journaled.

### Policy and observability

- OPA/Rego evaluation remains outside the pure model. A policy adapter returns
  only the normalized decision, revision, decision ID, obligations and expiry.
  Decision-log input/result may contain private data and is never copied into
  the PO journal.
- OpenTelemetry log records retain source timestamp and observed timestamp,
  resource, severity and optional trace/span IDs. Trace context is correlation,
  not identity or authorization.
- Prometheus/observability adapters consume reviewed recording/SLO rule outputs,
  not arbitrary unbounded user queries. They preserve evaluation interval,
  sample window, missing evaluations, request volume and low-traffic limits.
  Multiwindow burn evidence indicates impact; it does not prove root cause.

## Preflight and one-use action lifecycle

### Fixed review order

Preflight is one immutable snapshot in this order:

1. environment and identity;
2. exact typed executable/API action, argv and targets;
3. observed reason and contradictions;
4. target count, dependency/blast-radius estimate and expected impact;
5. exact permission and remaining admission limitations;
6. GitOps owner/reconciliation and desired revision;
7. organization policy, obligations, change window, approval/ticket and JIT;
8. verification predicate, stabilization window, timeout and observation source;
9. recovery option and limitations; and
10. `Cancel`, then the least-authoritative allowed positive action.

Before PO6 the positive action is `Insert reviewed command`; it never emits
Enter. After separate PO6 activation, `Execute reviewed action` may appear only
when every gate is eligible and the existing broker accepts the one-use grant.

### States and transitions

```text
draft
  -> eligible | refused
eligible
  -> review-ready | expired
review-ready
  -> awaiting-external-prerequisite | confirmed | cancelled | expired
awaiting-external-prerequisite
  -> review-ready(new revision) | cancelled | expired
confirmed
  -> revalidating
revalidating
  -> granted | refused | expired
granted
  -> starting | cancelled | expired
starting
  -> running | failed | cancelled | uncertain
running
  -> observing | failed | cancelled | uncertain
observing
  -> stabilizing | failed | cancelled | uncertain
stabilizing
  -> verifying | regressed | cancelled | uncertain
verifying
  -> succeeded | failed | uncertain
failed | regressed | uncertain
  -> recovery-proposed | complete
recovery-proposed
  -> complete
succeeded | cancelled
  -> complete
```

There is no automatic transition from `recovery-proposed` to a recovery action,
and no action chains a second mutation. Every retry is a new reviewed operation.
Process exit zero is not success; the named verification predicate must pass
after its stabilization window. Lost observation produces `uncertain`, not
success.

### One-use grant

The grant binds operation ID, route, passport/evidence/policy/preflight digests,
exact executable identity or API operation, argv, environment allowlist, target
UIDs, capability principal, authorization result, expiry, target-count ceiling,
timeout, verification/recovery profiles, and broker generation. It is consumed
once, expires after 15 seconds, and is invalidated by any context, target,
permission, policy, GitOps, credential, executable or broker change. It carries
no shell string and no implicit Enter.

## Journal, export, and privacy

### Session record

The content-minimized `ActionReceipt` contains only:

- schema, operation ID, route/passport digest, action ID, rule ID and target
  public identity digest;
- environment class, risk class, provider and adapter/profile versions;
- preflight, policy and grant digests plus public decision IDs;
- start/end time, final state, cancellation/timeout reason;
- verification/recovery profile IDs and outcomes; and
- process/provider ownership references needed for cleanup.

It excludes argv values by default, command output, terminal text/history,
selected text, raw logs, metric samples, provider responses, Kubernetes object
bodies, YAML, Terraform plan/state, environment variables, paths, labels that
have not passed a public allowlist, credentials, tokens, certificates, Secret
values, and policy input/result.

The session journal is memory-only by default: 256 receipts or 2 MiB, newest
valid entries retained. Optional persistence is explicitly enabled only after
PO5, uses the existing private no-follow atomic storage owner, and is bounded to
4,096 receipts or 16 MiB with a user-selected 1-30 day retention. The default is
zero persistent days. Corrupt/truncated/current-or-previous generation recovery,
read-only/disk-full behavior, concurrent access, atomic replacement, permission
repair, export preview, exact deletion and uninstall are required before
activation.

Export supports reviewed Markdown and strict JSON. It previews every included
field, redaction, unresolved unknown, and source reference. Export never embeds
provider URLs containing credentials, raw log text, terminal content, secret
values, or credential-cache paths.

## Planned configuration and actions

These names are reserved design candidates only. They must not be added to the
current configuration reference or action registry until their phase ships.

```toml
[production-operations]
enabled = false
mode = "read-only"

[production-operations.context]
adapters = []
scopes = []
show-freshness = true

[production-operations.suggestions]
enabled = false
max-visible = 5

[production-operations.investigation]
change = false
compare = false
network = false
slo = false
live-logs = false

[production-operations.safety]
unknown-environment = "production"
managed-actions = false
external-policy-profile = ""

[production-operations.incident]
enabled = false
persistence = "session"
retention-days = 0

[production-operations.managed-sessions]
port-forward = false
active-probe = false
debug-workload = false
allow-detach = false
loopback-only = true
```

Rules:

- omission preserves all current behavior;
- `enabled=false` is the default and removes every PO surface/source;
- enabling PO initially permits read-only context only; subfeatures are separate;
- safe ceilings, hard gates, production-unknown behavior, no-Enter, loopback,
  redaction and external authority are not user-weakenable;
- adapter entries are registered IDs and opaque profile references, never tokens
  or arbitrary executable paths;
- configuration reload is atomic and keeps the complete last-known-good state;
- a changed scope, adapter, policy profile, persistence, managed-action or
  managed-session setting invalidates affected candidates and grants;
- disabling cancels/joins work and clears memory; removal previews only exact
  Automexia-owned files.

No global default shortcut is added. PO3 uses the existing explicit CP5
invocation. Future command-palette/bindable actions are
`RefreshProductionContext`, `OpenProductionSituation`,
`ReviewOperationalCandidate`, `ToggleIncidentMode`, and
`StopManagedOperation`. They enter the action registry only with their owning
phase. User bindings follow the existing collision and last-known-good rules.
Inside a list, arrows move focus, Enter or the existing insertion key reviews or
inserts according to state, Escape cancels, and no key both selects and executes
a managed mutation.

## Accessibility contract

- Situation choices use a listbox only when each option is a single selectable
  value. Candidate details with nested controls use a grid/list-detail pattern,
  not interactive controls hidden inside an option.
- DOM/semantic focus and selection are distinct. Up/Down move within the list;
  Home/End are provided for more than five rows; type-ahead applies when useful;
  Tab leaves the component.
- A true managed confirmation is modal: focus moves inside, Tab is trapped,
  Escape closes, a visible cancel/close control exists, and focus returns to the
  invoker. For a long structured review, initial focus is the heading/summary,
  not the destructive action. The least destructive action is first.
- Focus remains visible and unobscured, state does not change merely on focus,
  and color/icon has textual redundancy. WCAG 2.2 focus, contrast, target-size,
  keyboard, status-message, reduced-motion and reflow/scale requirements are
  tested in the native renderer and accessibility tree.
- Refresh, stale, blocked and final-operation changes are coalesced meaningful
  status announcements. Rapid watch events are not individually announced.

## Resource and lifecycle limits

The numeric values in the machine contract are hard proposal ceilings. Important
interpretation:

- ordinary typing, PTY input/output, resize, render and startup perform zero PO
  filesystem, credential, provider, policy, process or network work;
- one active request plus one latest queued generation exists per pane;
- provider reads are globally fair and bounded; adapter/provider lower quotas
  win, including the CloudTrail limit;
- watches are shared only for the exact authorized scope and profile, parked
  when unused, relisted on loss, and joined on disable/logout/route close;
- caches contain neutral public records, never raw log bodies, metric series,
  Terraform plan/state, policy bodies or credentials;
- all failure paths close streams, processes, listeners, temporary debug
  resources and storage generations owned by that operation; and
- CP1 remains usable before, during and after every failure or uninstall.

Ceilings may be tightened from measured evidence. Raising one requires an ADR or
machine-contract revision, resource benchmark, abuse test and migration review.

## Traceability and implementation slices

| Requirement | Owner | First phase | Required independent evidence |
|---|---|---:|---|
| Passport, environment class, lock and context diff | pure model + app composition + D2/provider adapters | PO1 | exact provider public identity; pane/tab/window/clone isolation; context-change oracle |
| Evidence envelope, quality and freshness | pure model + each adapter | PO2 | provider response/UID/resourceVersion, source clock/gap, stale/relist/revoke oracle |
| Change, ownership, drift, cohort, network and SLO findings | DevOps rules + adapters | PO2 | provider/GitOps/observability native result; false-cause and coverage oracle |
| Kubernetes situation rules and stable ranking | DevOps rules + CP5 projection | PO3 | independent table/reference ranker, exact editor bytes and no Enter |
| Policy precedence and immutable preflight | app composition + external adapters | PO4 | live policy/IAM/RBAC/GitOps decision and forbidden-side-effect oracle |
| Incident state, log fan-in and journal | app composition + private storage + DN references | PO5 | source sequence/gap, zero-write default, storage tree/digest and privacy canaries |
| Action lifecycle and diagnostic sessions | existing broker + provider adapter | PO6 | external resource/process/listener state, no second action, cleanup and verification |
| Declarative organization packs | D7 trust/store + pure parser | PO7 | signature/digest/revocation, malicious pack, conflict and uninstall |
| New adapters and optional reorder-only model | independent adapter/model owners | PO8 | provider fixtures, deterministic fallback, model differential and resource evidence |

Every feature-matrix and reinforcement entry must link to its exact requirement,
fixture, checker, source owner, native evidence and external gate. A file name,
test count, mock return value, screenshot or documentation sentence is not proof
that an operation exists or is safe.

## Migration, rollback, disable, and uninstall

1. A new public record major version is read side by side with the old version.
2. Parsing and semantic validation complete before publication.
3. The new generation is written atomically where persistence is enabled.
4. The prior valid generation is retained for one rollback.
5. Interrupted, corrupt, unsupported or over-limit data leaves the last-known-
   good generation active and reports a redacted actionable error.
6. Downgrade refuses incompatible newer state; it never drops safety fields.
7. Disable revokes grants, cancels and joins work, closes sessions, removes PO
   projections, clears memory and returns to CP1 without restart.
8. Uninstall removes only the previewed Automexia-owned PO journal/pack/cache
   generations. It preserves config unless explicitly requested, provider
   credentials/configuration, kubeconfig, shell history, logs, user files,
   terminal settings and other extensions.

## Primary sources and resulting decisions

The sources below were reviewed on 2026-08-26. They constrain the design but do
not prove implementation or provider behavior in an Automexia build.

### Data contracts

- [JSON Schema 2020-12 Core](https://json-schema.org/draft/2020-12/json-schema-core)
  and [Validation](https://json-schema.org/draft/2020-12/json-schema-validation)
  support versioned structural contracts; Automexia additionally rejects
  duplicate keys and applies byte/cross-field limits.
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259),
  [RFC 8785 JCS](https://www.rfc-editor.org/rfc/rfc8785),
  [RFC 3339](https://www.rfc-editor.org/rfc/rfc3339),
  [TOML 1.0](https://toml.io/en/v1.0.0), and
  [SemVer 2.0.0](https://semver.org/spec/v2.0.0.html) define the proposed
  interchange, digest, time, user-config and compatibility rules.

### Kubernetes and OpenShift sources

- Kubernetes [API concepts](https://kubernetes.io/docs/reference/using-api/api-concepts/)
  define list/watch, resourceVersion, bookmarks and `410 Gone`.
- [Authorization](https://kubernetes.io/docs/reference/access-authn-authz/authorization/)
  and [authentication](https://kubernetes.io/docs/reference/access-authn-authz/authentication/)
  define exact current-user review, effective identity and ExecCredential
  expiry limitations.
- [Owners and dependents](https://kubernetes.io/docs/concepts/overview/working-with-objects/owners-dependents/),
  [Deployments](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/),
  [Pod lifecycle](https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/),
  [EndpointSlices](https://kubernetes.io/docs/concepts/services-networking/endpoint-slices/),
  and [NetworkPolicy](https://kubernetes.io/docs/concepts/services-networking/network-policies/)
  constrain ownership, rollout, symptom and path rules.
- [Service debugging](https://kubernetes.io/docs/tasks/debug/debug-application/debug-service/),
  [running-Pod debugging](https://kubernetes.io/docs/tasks/debug/debug-application/debug-running-pod/),
  [port forwarding](https://kubernetes.io/docs/tasks/access-application-cluster/port-forward-access-application-cluster/),
  and [kubectl version skew](https://kubernetes.io/releases/version-skew-policy/)
  constrain diagnostic-session and compatibility profiles.
- [`kubectl diff`](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_diff/)
  and [`rollout restart`](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_rollout/kubectl_rollout_restart/)
  show why preview support is command-specific and external-diff/secret behavior
  must be controlled.
- [`kube-rs`](https://github.com/kube-rs/kube) and its
  [architecture](https://kube.rs/architecture/) support the conditional
  list/watch adoption decision; the dependency remains unapproved.
- The Red Hat [OpenShift CLI reference](https://docs.redhat.com/en/documentation/openshift_container_platform/latest/html/cli_tools/openshift-cli-oc)
  supports retaining `oc` compatibility as an independently tested profile,
  rather than assuming all Kubernetes command behavior is interchangeable.

### GitOps, IaC, policy, and observability

- Argo CD [sync windows](https://argo-cd.readthedocs.io/en/stable/user-guide/sync_windows/),
  [resource tracking](https://argo-cd.readthedocs.io/en/latest/user-guide/tracking_strategies/),
  and [diff customization](https://argo-cd.readthedocs.io/en/stable/user-guide/diffing/)
  keep desired/live and reconciliation semantics in Argo.
- Flux [Kustomizations](https://fluxcd.io/flux/components/kustomize/kustomizations/)
  and [`flux reconcile kustomization`](https://fluxcd.io/flux/cmd/flux_reconcile_kustomization/)
  distinguish observation from mutation.
- Helm [`upgrade`](https://helm.sh/docs/helm/helm_upgrade/), Terraform's
  [JSON format](https://developer.hashicorp.com/terraform/internals/json-format),
  [`show`](https://developer.hashicorp.com/terraform/cli/commands/show), and
  [`plan`](https://developer.hashicorp.com/terraform/cli/commands/plan) establish
  preview versioning and sensitive-plan restrictions.
- OPA's [REST API](https://www.openpolicyagent.org/docs/rest-api) and
  [decision logs](https://www.openpolicyagent.org/docs/management-decision-logs)
  support normalized decisions and content-minimized logging.
- OpenTelemetry's [log data model](https://opentelemetry.io/docs/specs/otel/logs/data-model/),
  Prometheus [recording rules](https://prometheus.io/docs/prometheus/latest/configuration/recording_rules/),
  Google SRE [multiwindow burn-rate guidance](https://sre.google/workbook/alerting-on-slos/),
  and W3C [Trace Context](https://www.w3.org/TR/trace-context/) constrain time,
  coverage, impact and correlation claims.

### Cloud providers

- AWS [STS `GetCallerIdentity`](https://docs.aws.amazon.com/STS/latest/APIReference/API_GetCallerIdentity.html),
  [CloudTrail Event history](https://docs.aws.amazon.com/awscloudtrail/latest/userguide/view-cloudtrail-events.html),
  [CLI lookup limits](https://docs.aws.amazon.com/awscloudtrail/latest/userguide/view-cloudtrail-events-cli.html),
  [IAM simulation limitations](https://docs.aws.amazon.com/IAM/latest/UserGuide/access_policies_testing-policies.html),
  [IAM Identity Center](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html),
  and [temporary elevated access](https://docs.aws.amazon.com/singlesignon/latest/userguide/temporary-elevated-access.html)
  constrain the AWS profile.
- Azure [`az account`](https://learn.microsoft.com/en-us/cli/azure/account?view=azure-cli-latest),
  [ARM what-if](https://learn.microsoft.com/en-us/azure/azure-resource-manager/templates/deploy-what-if),
  [RBAC](https://learn.microsoft.com/en-us/azure/role-based-access-control/overview),
  [Activity Log](https://learn.microsoft.com/en-us/azure/azure-monitor/platform/activity-log),
  and [PIM activation](https://learn.microsoft.com/en-us/entra/id-governance/privileged-identity-management/pim-how-to-activate-role)
  constrain the Azure profile.
- Google Cloud [named configurations](https://cloud.google.com/sdk/gcloud/reference/topic/configurations),
  [`testIamPermissions`](https://cloud.google.com/iam/docs/testing-permissions),
  [Audit Logs](https://cloud.google.com/logging/docs/audit),
  [Cloud Asset history](https://cloud.google.com/asset-inventory/docs/get-asset-history),
  and [Privileged Access Manager](https://cloud.google.com/iam/docs/pam-overview)
  constrain the GCP profile.

### Accessibility and package trust

- W3C's [listbox](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/) and
  [modal dialog](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
  patterns plus [WCAG 2.2](https://www.w3.org/TR/WCAG22/) constrain selection,
  focus, confirmation and native evidence.
- [Sigstore verification](https://docs.sigstore.dev/cosign/verifying/verify/),
  the [Sigstore bundle](https://docs.sigstore.dev/about/bundle/), and
  [The Update Framework](https://theupdateframework.io/spec/) support reusing
  D7 provenance and distribution rather than creating a PO-specific trust path.

## Related authorities

- [Situation-Aware Production Operations](SITUATION-AWARE-PRODUCTION-OPERATIONS.md)
- [UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md)
- [Testing and evidence contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md)
- [ADR 0034](adr/0034-situation-aware-production-operations.md)
- [Architecture](ARCHITECTURE.md)
- [Build, wrap, and adopt](BUILD-WRAP-ADOPT-ARCHITECTURE.md)
- [Command productivity](COMMAND-PRODUCTIVITY.md)
- [Roadmap](ROADMAP.md)
- [Phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md)
