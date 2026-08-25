# Situation-Aware Production Operations

## Status and promise

This is the canonical proposal for Automexia's planned situation-aware
production operations track, `PO0-PO8`.

The detailed surface, interaction, responsive-layout, language, accessibility,
and module-by-module implementation contract is
[Situation-Aware Production Operations UX and Implementation Blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md).
The exact proposed record formats, freshness tables, rule profiles, ranking,
provider behavior, policy precedence, lifecycle, settings, migration,
dependency decisions, and requirement traceability are in the
[PO0 contracts and 2026 implementation decisions](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md).

- `PO0` is partially complete only at the documentation and research boundary.
- `PO1-PO8` are not implemented.
- No current setting, shortcut, completion source, provider adapter, cluster
  watcher, investigation view, live-log controller, incident mode, managed
  diagnostic session, policy pack, journal, model, or execution authority is
  created by this document.
- [ADR 0034](adr/0034-situation-aware-production-operations.md) is proposed,
  not accepted. A versioned **proposed, non-activating** machine contract and
  mutation-tested checker now exist; protected acceptance of that exact digest,
  failing real-path tests, owner review, and the phase gates below are still
  required before production code.

The product goal is simple: help an operator understand the environment they
are touching, find the safest useful next action, and complete that action with
less searching and less context switching. Automexia must show why an action is
suggested, what evidence it used, how fresh that evidence is, what the action can
affect, and how to recover. The operator remains in control.

Automexia cannot honestly promise to know *exactly* what must be done in every
incident. Telemetry can be incomplete, dependencies can be undocumented, and
several actions can be reasonable. When evidence is weak or contradictory, the
correct product behavior is to recommend diagnosis, request missing context, or
say that it cannot make a safe recommendation. Evidence quality must expose its
source authority, freshness, coverage and contradiction; it is never a claim of
certainty or an unexplained score.

## Why this belongs in Automexia

Ordinary completion answers “what text is valid here?” Production work needs
four additional answers:

1. Which account, cluster, namespace, identity, and incident am I operating on?
2. What is unhealthy, what owns it, and what depends on it?
3. Which valid next actions are relevant, safe, permitted, and useful now?
4. What happened after the chosen action, and should I continue, stop, or
   recover?

This fits Automexia's terminal-first direction because it shortens a workflow
without hiding the command, replacing the user's tools, or forcing a model or
paid service into the terminal. Kubernetes, cloud, GitOps, observability, and
organization-specific knowledge remain optional extension capabilities. The
core terminal remains useful, fast, and complete when the entire `PO` track is
disabled or uninstalled.

## Current evidence ledger

The implementation must extend existing owners instead of creating a second
completion system, provider cache, process runner, or environment authority.

| Current foundation | Status and reusable boundary | What `PO` must not infer from it |
|---|---|---|
| CP1 shell-native completion | Fully implemented locally and remains the default fallback | It does not provide live incident reasoning or production action ranking. |
| CP5 suggestion bridge and UI | CP5.1-CP5.4 are source-complete; CP5.5/CP5.6 activation and release evidence remain partial | Preview-disabled source is not permission to query providers or execute actions. |
| D2 Environment Capsules | Immutable, route-scoped context, freshness, cancellation, and isolation exist | A cached capsule is not proof that live provider state or authorization is current. |
| D6/M7-M12 providers | Bounded provider sources and cached product review exist locally; execution remains nonactivated | They do not yet expose a production evidence graph, broad watches, credentials, or provider execution. |
| M13/CP4 provider actions | Typed, insert/copy-only, route-bound actions with final revalidation and production confirmation exist locally | They do not diagnose incidents or authorize a new execution path. |
| D3/M3-M5 exact-argument runner | A fail-closed, nonactivated exact-argv path and lifecycle owner exist | `PO` cannot activate, duplicate, or bypass that runner. |
| DN0 diagnostic navigation | Documentation-only proposal for navigating terminal error sections | Log navigation is not infrastructure diagnosis and does not grant log or history access to extensions. |
| Existing prompt context strip | Shows route-scoped Git, container, Kubernetes, cloud, Terraform, OS, and user context | Displayed context can be stale and is not a production lock or permission result. |

The missing product owner is a bounded operation-intelligence layer that joins
these existing contracts without moving provider or policy work into the
renderer, pseudo-terminal (PTY), input, resize, or startup paths.

## Goals

The planned feature set includes all of the following:

- a pane-local production environment passport and context lock;
- bounded change/provenance intelligence that distinguishes correlation from
  proven cause and preserves field/GitOps ownership;
- typed resource-state explanation for unhealthy, pending, degraded, and
  partially available resources;
- normalized healthy-cohort, revision/canary, and later multi-environment
  comparison;
- staged network-path diagnosis from read-only topology through separately
  reviewed active probes;
- optional user-impact, log-fan-in, incident-time, and handoff views that reuse
  existing observability and diagnostic owners instead of becoming a dashboard;
- situation-aware command completion for Kubernetes, cloud, GitOps, and other
  DevOps/SRE tools;
- evidence-based prioritization with reasons, freshness, provenance,
  completeness, contradiction, explicit unknowns, and clear refusal states;
- a dependency graph and bounded impact projection that never claims complete
  blast-radius proof;
- permission, policy, change-window, GitOps, and just-in-time (JIT) access
  awareness;
- an impact preflight that shows exact command arguments and affected targets;
- Incident Mode with pinned context, a bounded timeline, and shared recovery
  intent;
- an explicit execute, observe, stabilize, verify, recover lifecycle;
- separately gated managed diagnostic sessions for Kubernetes port forwards,
  controlled network probes, and safe workload debugging;
- signed and versioned organization runbook/policy packs;
- cross-tool guided workflows that preserve each tool's native command and
  authority boundary;
- a redacted, bounded local action journal that complements provider audit
  records; and
- optional provider adapters and, only in a later phase, an optional small local
  ranker that cannot invent or execute commands.

## Non-goals and hard boundaries

`PO0-PO8` must not introduce:

- an autonomous production agent or automatic remediation loop;
- implicit Enter, background mutation, or an action executed from a completion
  result;
- an LLM dependency in the DevOps/SRE extension or terminal core;
- a claim that a single score proves root cause, business priority, or safety;
- direct provider logic, credentials, cluster watches, or policy evaluation in
  the terminal core, renderer, PTY, or key-processing path;
- shell command strings, `sh -c`, `cmd /c`, PowerShell expression evaluation,
  or another process runner;
- credential, token, kubeconfig secret, log body, metric series, or terminal
  history persistence in the operation cache or journal;
- global `kubectl`, cloud CLI, GitOps, shell, or provider configuration changes;
- a replacement for Kubernetes admission, role-based access control (RBAC),
  cloud authorization, GitOps controls, change management, or provider audit
  logs;
- a replacement for the Kubernetes scheduler, GitOps reconciliation,
  observability storage, an SLO system, or a service-mesh/network control plane;
  or
- a mandatory extension. Disabling or uninstalling `PO` returns immediately to
  CP1 and the existing terminal behavior.

## User experience

The experience must stay simpler than the systems it coordinates. Only the
compact environment passport is persistent. The user explicitly requests the
situation list, opens detail only when needed, and enters production preflight
for a deliberate reviewed action. Monitoring remains attached to that one
operation. Incident Mode is opt-in. The complete interaction rules, responsive
layouts, copy, failure states, accessibility behavior, and implementation map
are owned by the
[UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md).

The shortest useful flow is:

```text
see context -> request completion -> select a read-only command -> keep typing
```

The managed production flow is deliberately longer:

```text
see locked context -> request/review action -> preflight -> approve once
  -> execute once -> monitor -> verify or recover
```

Each surface has at most one emphasized primary action and a visible safe exit.
Refresh never steals focus, changes the command buffer, opens a dialog, or
blocks ordinary typing.

### Production environment passport

The existing context strip above the command remains the compact always-visible
surface. When `PO` is installed and enabled, it may add a route-scoped passport
whose fields are present only when observed:

- provider and account, tenant, subscription, or project;
- region and zone;
- cluster, Kubernetes context, namespace, and selected workload scope;
- authenticated public identity and role, without tokens or group dumps;
- credential or elevation expiry;
- environment class such as local, development, staging, or production;
- GitOps owner, application, synchronization state, and applicable sync window;
- incident identifier and current operator role; and
- freshness, source, and last successful observation.

Production is never expressed by color alone. The strip uses text and an icon,
and the production state remains visible while a completion list, detail drawer,
review, or monitor is open.

### Pane-local context lock

An operator can lock a pane to the exact passport revision. A lock:

- belongs to one route and never leaks to another pane, tab, or window;
- records public identifiers and opaque credential references, never secrets;
- detects account, region, cluster, namespace, identity, GitOps application, or
  incident changes;
- blocks production mutation review when required context changes or expires;
- shows an exact before/after context diff and requires explicit adoption of the
  new revision; and
- clears on route destruction, logout, provider revocation, uninstall, or an
  explicit user action.

The lock does not freeze the external environment. It freezes the identity
against which evidence and review must be revalidated.

### Situation-aware completion

Completion keeps the user's editor buffer as the source of truth. It operates in
three layers:

1. CP1 provides ordinary native syntax and shell completion.
2. CP5 provides a bounded editor bridge, candidate surface, and replacement-only
   insertion.
3. `PO` may add typed operational candidates built from a previously refreshed,
   route-scoped evidence snapshot.

Each operational candidate contains:

- the native command text that would be inserted, never an executable callback;
- the exact executable identity and argument array when known;
- target, environment, and risk labels;
- a short reason such as “three unavailable replicas owned by this Deployment”;
- supporting and contradicting evidence with source and age;
- an evidence-quality summary covering freshness, provenance, agreement,
  completeness, and scope coverage, without an unexplained percentage;
- explicit `known`, `unknown`, `unavailable`, `unsupported`, `stale`,
  `redacted`, or `not applicable` states for safety-relevant facts;
- prerequisites and unavailable evidence;
- expected effect, blast radius, and verification signal;
- safer diagnostic, GitOps, or rollback alternatives; and
- an expiry bound to route, context, evidence, policy, and candidate generation.

Selecting an ordinary insertion candidate replaces only the authenticated
editor span. It does not press Enter. Automexia cannot enforce policy over a
command the user types or runs directly in the native shell, and the UI must not
claim that it can. A mutating PO candidate is therefore either clearly marked
as an advisory/manual shell insertion or, when policy requires enforcement,
exposed only through `Review managed action`. Every Automexia-managed external
change requires a separate explicit preflight and one-use review. The
[UX blueprint's manual-versus-managed contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md#advisory-shell-path-versus-managed-path)
defines the exact wording and controls.

### The requested Kubernetes rollout example

When the user types:

```text
kubectl rollout
```

Automexia may show ordinary valid subcommands and a separate “Current situation”
group. It must not blindly place `restart` first. A planned deterministic flow
is:

1. Parse the current buffer and preserve the user's flags and selected context.
2. Freeze the pane passport and current evidence generation.
3. Read a bounded namespace-scoped snapshot of workloads, Pods, Events, rollout
   conditions, service endpoints, and permitted observability summaries.
4. Resolve `Pod -> ReplicaSet -> Deployment` or other controller ownership by
   `metadata.ownerReferences` kind, name, and UID. Never guess ownership from a
   Pod name.
5. Classify the symptom before proposing mutation:
   - a transient or stale process may make `rollout restart` relevant;
   - a bad image, missing configuration, failed scheduling, quota, storage,
     probe, permission, dependency, or current bad rollout usually requires a
     different action;
   - an active or stalled rollout may make `status`, `history`, `pause`, `resume`,
     or `undo` more relevant than `restart`;
   - weak or conflicting evidence produces diagnosis, not remediation.
6. Reject candidates that fail context, freshness, permissions, policy, change
   window, GitOps, target, or blast-radius gates.
7. Rank the remaining candidates with explicit lexicographic dimensions rather
   than one unexplained score.
8. Show the affected Deployment, why it is prioritized, what contradicts the
   recommendation, and what would change the ranking.
9. Insert the selected native command without Enter.
10. Before review or execution, refresh permissions and mutable evidence and
    invalidate the candidate if its bound revision changed.

Example candidate presentation:

```text
Current situation — prod / eu-west / payments

1  kubectl rollout status deployment/payments-api -n payments
   Diagnose first · rollout is already progressing · fresh 2 s

2  kubectl rollout undo deployment/payments-api -n payments
   High risk · new revision correlates with 8/8 unavailable replicas
   Requires GitOps exception and production approval

3  kubectl rollout restart deployment/notification-worker -n payments
   Medium risk · 4 crash-looping Pods · restart may clear stale connections
   Evidence partial · configuration cause not excluded
```

If the evidence instead shows `ImagePullBackOff`, an invalid Secret reference,
or unschedulable Pods, Automexia must not present restart as the preferred fix.

### Evidence and priority

The first release uses deterministic adapters and rules. A candidate passes hard
gates before ranking:

- exact route, passport, namespace, resource UID, and generation match;
- sufficiently fresh required evidence;
- supported tool, resource kind, action, and exact argument contract;
- current authorization and extension capability;
- organization policy and change-window allowance;
- GitOps ownership and drift strategy;
- valid rollback or explicit no-rollback disclosure for production mutation;
- bounded target count and blast radius; and
- no unresolved high-severity contradiction.

Candidates that pass are ordered lexicographically by:

1. expected safety and reversibility;
2. causal evidence strength rather than symptom count alone;
3. service criticality and incident objective from reviewed policy/runbooks;
4. dependency ordering and reduction of customer impact;
5. urgency and duration of the observed failure;
6. evidence freshness and completeness;
7. operator and organization preferences; and
8. deterministic stable tie-breakers.

Business criticality never comes from guessed names or popularity. It comes from
reviewed organization data, service catalogs, runbooks, incident scope, or an
explicit operator choice. Missing criticality remains `unknown`.

### Investigation capability families

The requested features are delivered as six reusable capability families rather
than fifteen independent engines, controllers, caches, and surfaces. The table
below is the canonical scope map; the UX blueprint owns exact interaction and
the roadmap owns delivery status.

| Family | Recurring operator question | Included capabilities | Earliest phase |
|---|---|---|---|
| Provenance and change | What changed, who owns this field, and will reconciliation undo me? | Change Intelligence, field ownership, desired/live drift | PO2 |
| Resource explanation | Why is this resource unhealthy or pending? | Kind-specific explainers, scheduling and capacity diagnosis | PO2 |
| Comparison | What differs from a healthy control? | Healthy cohort, revision/canary, later region/cluster comparison | PO2 for one-scope cohorts; PO8 for cross-environment fan-out |
| Network diagnosis | At which layer is this path failing? | DNS, Service, EndpointSlice, backend, policy, TCP, TLS, protocol; active probes later | PO2 read-only; PO6 active probes |
| Incident evidence | Are users affected, where is the first failure, and what do I hand over? | SLO lens, bounded log fan-in, incident-time navigation, hypotheses, handoff | PO2 summaries; PO5 incident surfaces |
| Managed diagnostic sessions | How do I inspect or reach the workload safely? | Kubernetes port forwards, controlled probes, ephemeral/copy/node debugging | PO6 and separately activated provider/D3 capabilities |

These families share `ResourceRef`, provenance, evidence quality, environment
binding, typed actions, safety decisions, limits, and lifecycle contracts. They
do not share provider handles, mutable UI state, process ownership, or execution
authority.

#### Change Intelligence, ownership, and drift

`What changed?` correlates a bounded time window around a trusted incident or
diagnostic anchor. Initial sources are Kubernetes rollout revisions, immutable
image digests, object generations, GitOps revisions/syncs, scaling and relevant
provider events. CI/CD, cloud-change and ticket sources are separate adapters.

Every change records target, source, provider timestamp, normalized timestamp,
clock-quality/skew state, before/after content-minimized fingerprint, actor class
when policy permits it, and provenance. It never stores a full Secret, raw
manifest, provider payload, log, or terminal line. A mutable image tag is not a
stable change identity without its observed digest.

The UI says `revision 185 became ready 5m before the first failure`. It says
`caused` only when a separately reviewed rule has sufficient supporting and
counterfactual evidence. Kubernetes Events remain best-effort supporting data,
not the sole authority for cause or exact timing.

Field ownership uses bounded `metadata.managedFields`, GitOps ownership, and
provider-native provenance. A field manager is a software manager identity, not
proof of the human actor. Human attribution requires a separately authorized
audit source. Desired/live drift consumes GitOps-native diff, ignore, sync, and
self-heal semantics; Automexia does not invent desired state or reconciliation.

#### Explain Resource State and scheduling

`Explain` returns structured findings for each supported kind rather than one
generic prose answer. The initial Kubernetes slice should cover Pods,
Deployments, StatefulSets, DaemonSets, Services/EndpointSlices, Jobs/CronJobs,
Nodes, and PVCs. Each finding names observed facts, supporting and contradicting
evidence, unknowns, likely category, and safe read-only next actions.

Conditions, container/controller status, ownership, readiness, EndpointSlices,
and observed generations are primary when the resource actually publishes them.
Events and approved bounded log-category summaries may be supporting inputs with
independent freshness and retention; PO2 does not fetch or retain raw log bodies.
`CrashLoopBackOff`, exit code `137`, `Pending`, or `NotReady` describe
symptoms; they do not alone establish root cause.

Scheduling diagnosis interprets the Pod scheduling condition, scheduler
messages, requests, allocatable summaries, taints/tolerations, affinity,
topology, PVC binding, and approved autoscaler state. It does not reproduce the
cluster's scheduler plugins or claim an exact eligible/closest node unless a
provider-native authority supplies that result.

#### Healthy cohort and revision comparison

`Compare with healthy` selects a bounded cohort through a visible comparability
contract: same owner UID and workload role, declared environment, and either the
same revision or an explicitly selected control revision. It compares normalized
semantic fields, not raw YAML, and ignores expected identity fields such as Pod
name, UID, IP, creation time, resource version, and controller hash.

Meaningful dimensions include immutable image identity, configuration and Secret
references, safe generation metadata, resource limits, service account, security
context, node/zone, volumes, readiness, endpoints, restarts, conditions, and
runtime class. Secret values are never fetched or compared. One arbitrary healthy
Pod is not a cohort; the UI explains sample size, exclusions, freshness, and when
no trustworthy control exists.

Revision/canary comparison adds bounded request count, exposure duration, traffic
split, error ratio, latency, restart, and error-category summaries. It reports
disproportionate association, not automatic rollback authority. Cross-region or
cross-cluster comparison remains read-only and later: each environment is
independently authorized, and a reviewed service-catalog mapping must establish
that the workloads are comparable.

#### Network Path Diagnosis

The initial read-only path resolves destination and source scope, then inspects
DNS configuration, Service ports/selectors, EndpointSlice readiness, backend
Pods, applicable NetworkPolicy, and supported ingress/gateway/mesh metadata.
Each layer is `passed`, `failed`, `unknown`, `not tested`, or
`unsupported` with source and age. Configuration presence never masquerades
as a connectivity test.

Every active DNS, TCP, TLS, HTTP, database, or service-mesh probe must state its
vantage point: Automexia host, source Pod, diagnostic Pod, node, or external
location. A local-machine result is not evidence for a Pod-to-Service path.
Active probes are separately reviewed PO6 managed diagnostic actions with exact
target, image/executable, protocol, traffic/deadline limits, permissions,
cancellation, cleanup, and receipt. NetworkPolicy inspection cannot by itself
exclude CNI, node, cloud firewall, routing, proxy, mesh, or application failure.

#### SLO, logs, and incident time

An optional SLO lens reads reviewed existing SLO/recording-rule results and
shows objective, window, remaining budget, burn rate, request volume, user-facing
error ratio, source, and freshness. It does not store time series, calculate an
organization's authoritative SLO from arbitrary raw queries, or let high impact
bypass safety gates. Low traffic and missing service mapping remain visible
limitations.

Multi-source log fan-in is a PO5 memory-only live view, not part of the PO2
evidence cache. Each record retains source resource/container, source-local
sequence, provider timestamp when present, local arrival time, stream, bounded
bytes, decoding state, and gap/drop state. Cross-Pod clock order is approximate.
The renderer merges projections while Error Navigation keeps its own bounded
marker/navigation authority; neither receives one unbounded concatenated string.

Incident-time actions jump only to a trusted structured record, diagnostic
marker, or command anchor. `Closest known event` is labelled approximate when a
terminal line has no reliable timestamp. Timezone, source clock, and skew are
part of the evidence rather than silently normalized away.

### Risk language and confirmation

Risk is an independent policy result, not the ranking score:

| Tier | Examples | Interaction |
|---|---|---|
| Read only | status, describe, history, diff, logs with bounded redaction | Insert or run through the already approved read-only path when one exists. |
| Low-risk local | change local filter, open detail, pin incident context | Direct, reversible UI action. |
| External mutation | restart one non-production workload, trigger an approved sync | Exact preflight and confirmation. |
| Production mutation | restart, rollback, scale, fail over, modify cloud resources | Production lock, fresh authority/policy checks, impact preview, typed confirmation where policy requires it. |
| Destructive or broad | delete, prune, rotate, multi-target mutation, force conflict | Denied by default in early phases; later support requires a separate reviewed contract. |

Repeated confirmation never becomes automatic consent. Confirmation binds the
exact candidate digest, executable, arguments, route, passport, targets,
evidence generation, policy revision, expiry, and one use.

### Impact preflight

Before any Automexia-managed external mutation, the review shows:

- executable identity and exact argument array;
- account, region, cluster, namespace, resource kind/name/UID, and target count;
- current health and relevant dependencies;
- rollout strategy, desired/available replicas, `maxUnavailable`, `maxSurge`,
  and applicable disruption information;
- authorization result and credential/elevation expiry;
- GitOps owner, drift state, sync window, and whether reconciliation may revert
  a direct change;
- policy result, required approval, ticket, or incident reference;
- expected effect, uncertainty, and signs that the action is working;
- stop conditions, timeout, rollback path, and the consequence of no rollback;
  and
- the exact evidence that is stale, missing, or contradictory.

A preflight is not a fake dry run. Where a tool has a real server-side diff,
validation, or plan operation, the adapter may use it through approved
read-only/external-tool authority. Where it does not, the UI says “impact
estimate” and does not imply server validation. In particular, the current
Kubernetes `kubectl rollout restart` interface has no general dry-run guarantee.

Before PO6 activation, preflight may end only in cancellation or a clearly
labelled `Insert reviewed command` advisory path. After insertion, the native
shell owns execution and external state can change again. Only the separately
activated PO6 managed path may offer `Execute reviewed action`, perform final
revalidation, and enforce Automexia's monitor/receipt lifecycle.

### GitOps awareness

Direct cluster mutation can be overwritten by reconciliation or leave declared
and live state inconsistent. A GitOps adapter therefore identifies the owning
application, source revision, synchronization state, auto-sync/self-heal policy,
and applicable sync window when available.

The default ordering is:

1. prefer a reviewed source change and sync when that satisfies the incident;
2. use a direct emergency action only when policy explicitly permits it;
3. explain whether self-heal may revert the action;
4. bind any exception to incident, target, revision, expiry, and recovery plan;
5. verify both live health and declared-state reconciliation afterward.

Automexia does not become a GitOps controller and never silently disables
reconciliation.

### Just-in-time access awareness

The provider extension may expose public role state, eligibility, activation
requirements, and expiry through opaque credential references. Automexia can
explain that an action is unavailable until an approved temporary elevation is
active. It must not store credentials, approve its own elevation, bypass
multi-factor authentication, or turn an eligible role into standing access.

### Incident Mode

Incident Mode is an explicit pane/workspace state, not a background agent. It
provides:

- a pinned incident identifier, objective, severity, environment, and operator
  role;
- a stable evidence timeline containing summaries and source references, not raw
  logs or metric series;
- explicit hypotheses with supporting, contradicting, missing, and negative
  evidence whose source coverage and time window are known;
- a filtered suggestion set that favors diagnosis, containment, recovery, and
  verification relevant to the incident objective;
- visible actions attempted, results, timeouts, and invalidations;
- trusted or approximate time-navigation links and optional bounded live-log
  references without persisting the underlying stream;
- handoff/export of a redacted receipt bundle after explicit review; and
- a clear exit that releases watches, cancels generations, closes adapters, and
  returns to ordinary completion.

Incident Mode does not grant broader permissions. It can make high-risk actions
more visible only when policy allows them; it cannot lower their risk or
confirmation requirements.

### Execute, observe, stabilize, verify, recover

Execution belongs to the existing application-owned exact-argument broker and
remains unavailable until D3 and the relevant provider capability are activated.
When a reviewed action eventually runs, `PO` follows one bounded state machine:

```text
draft -> preflight -> awaiting approval -> final revalidation -> executing
      -> observing -> stabilizing
      -> verified | failed | uncertain | cancelled | recovery offered
      -> recovering -> recovered | recovery failed | recovery uncertain
```

Every transition is route- and generation-bound. Monitoring uses declared
success, failure, timeout, stabilization, and regression predicates. Before-state
capture is action-specific and content-minimized. `resourceVersion` or an
equivalent provider token guards launch-time concurrency but is not assumed to
remain unchanged after an intentional mutation. The operation never waits
forever, recursively recommends actions, or launches the next mutation
automatically. Missing observations or an expired deadline produce `uncertain`
rather than false success or failure.

The final launch uses a one-use `ManagedActionGrant`, not a pane-wide mutation
mode. It binds the exact action/target digest, route, environment, evidence,
policy and authorization generations, expiry, and one use. New panes, cloned
sessions, restored workspaces, or another action never inherit it.

### Managed diagnostic sessions

Kubernetes port forwards, active connectivity probes, and debug sessions reuse
the same managed-operation and D3/provider lifecycle; they do not introduce a
second runner, listener manager, or credential owner.

- Port forwards bind local endpoint, target kind/name/UID, resolved backend,
  environment, identity, owner route, and reconnect policy. Loopback is the
  default. Closing the owner stops the session unless the user deliberately
  reviews a detached background owner. Production follow/reconnect is off by
  default and revalidates before switching to a replacement backend.
- Active probes bind vantage point, destination, protocol, traffic/deadline
  ceilings, tool/image identity, permissions, and cleanup. They never run as an
  invisible consequence of opening an explanation.
- Workload debugging distinguishes an ephemeral container, copied Pod, and node
  debug. The review shows immutable image identity/provenance, profile,
  capabilities, process-namespace behavior, service-account/configuration/Secret
  exposure, admission result, cleanup, and reversibility. Ephemeral containers
  cannot be changed or removed after addition; debug copies can still reach
  production dependencies; node debugging is denied initially and requires its
  own later high-risk contract.

Arbitrary debug images, privileged profiles, public listeners, automatic
reconnect, and automatic cleanup claims are refused until their separate policy,
native, crash-recovery, and resource evidence passes.

### Organization runbook and policy packs

Teams may install signed, versioned, declarative packs that contribute:

- service identities, owners, dependencies, criticality, and environments;
- symptom and evidence predicates;
- typed candidate actions using registered commands and schemas;
- prerequisites, risk, blast-radius rules, approvals, and change windows;
- success, stop, timeout, and recovery predicates;
- links to human runbooks and tickets; and
- compatibility and expiration metadata.

Packs are data, not arbitrary scripts. They cannot add shell evaluation, direct
network access, credentials, capabilities, or execution. Installation, update,
disable, rollback, and uninstall use the existing signed ecosystem and exact
grant principles only after their future activation gates are satisfied.

### Cross-tool guided workflows

One incident often spans Kubernetes, Prometheus-compatible telemetry, GitOps,
cloud load balancers, DNS, identity, and ticketing. `PO` normalizes observations
and candidate actions but does not hide their source. A workflow step always
names its owning adapter and native command or API operation.

The initial workflow boundary is guided and one-step-at-a-time:

1. collect bounded read-only evidence;
2. propose one typed action or diagnostic;
3. review and, where authorized, execute it;
4. monitor declared signals;
5. return to the operator before another mutation.

Multi-step unattended plans belong neither to `PO` nor the DevOps/SRE extension.
The optional LLM Orchestration extension remains a separate future product and
cannot bypass this policy.

### Action journal and provider audit

The journal records content-minimized receipts:

- incident and operation identifiers;
- public environment and target identity;
- command/action digest and redacted display summary;
- evidence and policy revision digests;
- approval class and public authority outcome;
- start/end time, final state, verification, and recovery outcome.

It excludes command output, logs, metric samples, selected terminal text,
credentials, tokens, environment values, raw provider responses, and arbitrary
arguments that may contain secrets. The session journal is the default; protected
local persistence is opt-in. Provider and Kubernetes audit records remain the
authoritative record of external operations.

## Architecture and ownership

### One owner per responsibility

| Responsibility | Proposed owner |
|---|---|
| Editor buffer, replacement span, insertion, CP1 fallback | Existing CP5/CP1 owners |
| Composition, route binding, user review, execution handoff | Desktop application |
| Pure evidence/action/policy types and deterministic ranking | New private renderer/PTY-independent operation model |
| DevOps/SRE rules, Kubernetes/cloud/GitOps interpretation | First-party DevOps/SRE extension and optional adapters |
| Provider collection and credential references | Existing provider extensions under explicit capabilities |
| Exact external execution and child lifecycle | Existing application ExternalToolRunner/session broker |
| UI geometry and rendering | Existing screen/renderer owners consuming immutable projections |
| Admission, RBAC, IAM, PIM, GitOps, and audit authority | External systems; Automexia only reports their current result |

The new pure model must not depend on a renderer, window, shell, PTY, provider
SDK, credential store, filesystem, process, network, model runtime, or extension
host. Provider adapters map bounded external observations into versioned neutral
records. The application composes those records and publishes an immutable UI
snapshot after stale-generation and route checks.

### Proposed typed contracts

The proposed names below are mirrored by the non-activating
[PO0 machine contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md#exact-neutral-records).
They remain unavailable product API until that exact contract is accepted:

```text
EnvironmentPassport
EvidenceSnapshot
Observation
KnowledgeState
EvidenceQuality
ChangeRecord
FieldOwnershipRecord
ResourceExplanation
CohortDefinition
ComparisonFinding
NetworkPathAssessment
SloImpactSummary
ResourceNode
ResourceEdge
OperationalIntent
SituationAssessment
Recommendation
CandidateAction
RiskAssessment
SafetyDecision
ImpactPreflightSnapshot
ManagedActionGrant
VerificationPredicate
OperationMonitor
ManagedDiagnosticSession
ActionReceipt
```

Every record has a version, bounded identifiers, source, observed time, expiry,
route, environment revision, generation, redaction class, and deterministic
digest. Unknown fields are either rejected or preserved only under an explicitly
versioned compatibility rule. Display strings are untrusted and never become
arguments. Executable identity and exact arguments are typed separately.

The provisional crate/module, app-controller, UI-model, screen, renderer, CP5,
and execution-broker ownership map is maintained in the
[UX blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md#proposed-implementation-map).
It is an implementation plan, not evidence those modules exist.

### Data flow

```text
explicit refresh / bounded watcher event
  -> provider adapter validates and normalizes public observations
  -> route-scoped snapshot and resource graph
  -> deterministic rules create assessments and recommendations
  -> registered typed actions plus hard safety/freshness gates
  -> lexicographic ranking and immutable UI projection
  -> CP5 replacement-only insertion
  -> explicit preflight and one-use confirmation
  -> existing exact-argv broker, when separately activated
  -> bounded observation and stabilization
  -> verification, recovery offer, redacted receipt
```

Publication happens before the renderer is woken. Cancellation, route closure,
context change, provider revocation, policy replacement, and shutdown invalidate
all affected generations before another result can be published.

### Provider adapter boundary

AWS, Azure, Google Cloud, Kubernetes/OpenShift, GitOps, observability, incident,
and service-catalog support are separately installable adapters. Each declares:

- exact public evidence types and required capabilities;
- credential/reference policy;
- scopes, selectors, list/watch or polling behavior;
- freshness and failure semantics;
- query, byte, object, time, concurrency, and retry limits;
- redaction and persistence policy;
- supported actions and independent authorization checks;
- disable, logout, revoke, uninstall, and cleanup behavior; and
- native/provider fixtures required before a support claim.

No adapter is required for ordinary terminal or CP1 completion. One slow or
failed adapter degrades only its own evidence and candidates.

## Resource and performance contract

These `PO0` numbers are provisional design ceilings mirrored by
`tests/fixtures/production-operations/po0-contract-v1.json`. They are frozen for
proposal review by its canonical digest and mutation checker. Acceptance must
approve or revise that exact machine contract; benchmarks may only tighten
accepted ceilings without a reviewed migration.

| Resource | Provisional ceiling/default |
|---|---|
| Typing path | No filesystem, provider, network, credential, policy, or process work per keystroke. Local parsing and cached lookup only. |
| Debounce | 100 ms for contextual request replacement; a newer editor generation cancels the older request. |
| Publication | Cached candidate target: 100 ms after explicit invocation; 500 ms hard contextual deadline. CP1 remains available immediately. |
| Active work | One active request per pane; one queued latest generation; 32 active routes process-wide. |
| Candidates | 32 operational candidates per request; 8 evidence references per candidate; 4 KiB per display field. |
| Frames | 64 KiB request and 256 KiB reply after decoding; oversize fails closed. |
| Scopes | Current namespace plus at most three explicitly pinned scopes. Cluster-wide collection is off by default. |
| Snapshot cache | 10,000 resource nodes or 32 MiB process-wide, whichever comes first; least-recently-used eviction never removes the active generation. |
| Network | Four concurrent evidence reads process-wide, two per adapter; default 5 queries/s with burst 10; adapter-specific lower limits win. |
| Freshness | Default 15 s for non-production and 5 s for production health evidence; mutation preflight always requests current authorization and required mutable evidence. |
| Logs and telemetry | No raw log body or time series in the cache. Adapters return bounded summaries and stable source references only. |
| Comparisons | At most 32 cohort resources, 4 revisions, or 4 explicitly selected environments per request; adapters may impose lower limits. Cross-environment access is independently authorized. |
| Live log fan-in | Off by default and PO5-only: 8 source streams per view, 16 process-wide, 20,000 retained records or 8 MiB per view, and 1 MiB/s per view. Over-limit data is dropped with a visible gap; memory is cleared on close. |
| Active probes | PO6-only and explicit: one active probe per route, two process-wide, 30 s default deadline, no automatic retry, exact cancellation and cleanup. Adapter-specific lower limits win. |
| Managed diagnostic sessions | Reuse D3/provider limits; initial PO ceiling is 4 Kubernetes port forwards and one debug session per route, 8 sessions process-wide. Detached/background ownership is explicit and production reconnect is off by default. |
| Journal | Session default: 256 receipts or 2 MiB. Optional protected persistence: 4,096 receipts or 16 MiB with retention and exact uninstall. |
| Retries | At most two jittered retries for eligible read-only transient failures; no automatic mutation retry. |
| Shutdown | Cancel work, close watches, revoke transient references, and join owned workers within a measured bounded deadline. |

Kubernetes collection should use an initial bounded list followed by watches with
`resourceVersion` and relist/recovery semantics. Watches are namespace- and
resource-scoped, shared only for identical authorized scopes, and parked when no
visible route or Incident Mode needs them. A bookmark is progress evidence, not
freshness by itself. Cache eviction, watch restart, `410 Gone`, authorization
loss, network loss, and provider throttling publish explicit degraded states.

Persistent storage is off by default for evidence. No disk cache is required to
produce a completion. Optional journal persistence uses the repository's private,
atomic, bounded, no-follow storage owner and has exact disable/uninstall cleanup.

## Failure and refusal behavior

| Condition | Required behavior |
|---|---|
| No adapter or feature disabled | CP1 and ordinary terminal operation continue unchanged. |
| Offline, throttled, or timed out | Keep last-known-good evidence visibly stale; prefer read-only diagnostics or refuse mutation. |
| Changed account/cluster/namespace/identity | Invalidate candidates, show context diff, and require explicit adoption. |
| Insufficient RBAC/IAM/PIM authority | Explain the missing permission or elevation state without requesting or storing credentials. |
| GitOps owner unknown | Show the uncertainty; do not claim a direct action is durable. |
| Policy unavailable or invalid in production | Fail closed for production mutation; read-only diagnosis can continue. |
| Conflicting evidence | Surface the conflict and what could resolve it; do not invent certainty. |
| Candidate or target stale | Remove or disable it; never silently retarget by name. |
| Monitor loses evidence | Stop automatic progression, mark verification unknown, and return control. |
| Log source disconnects or exceeds limits | Keep other sources live, mark a visible source gap/drop interval, and never present the merged order as complete. |
| Probe vantage point unavailable | Preserve read-only path inspection and say which layer was not tested; do not substitute a local probe silently. |
| Comparable peer or environment unavailable | Explain the missing cohort/mapping and omit comparative conclusions. |
| Debug/tunnel owner closes or authorization changes | Stop or fail closed according to the reviewed owner; no silent retarget, public listener, or orphan child remains. |
| Recovery unavailable | Say so before execution and in the receipt. |
| Adapter crash or uninstall | Cancel its work, remove its candidates and data, preserve core completion, and clean owned resources. |

## Security and privacy threat model

| Threat | Mandatory control |
|---|---|
| Cross-pane or stale-context action | Route/passport/resource-UID/generation binding plus final revalidation. |
| Command or argument injection | Strict typed actions, exact executable/argv arrays, control/bidi rejection, and display/argument separation. |
| Confused deputy | Independent external authorization, exact capability principal, one-use grant, and app-owned execution composition. |
| Poisoned logs, events, labels, telemetry, or runbooks | Treat all as untrusted bounded data; provenance and contradiction reporting; no text-to-command conversion. |
| Secret disclosure | Public-field allowlists, opaque credential references, redaction canaries, content-free receipts, and no raw evidence persistence. |
| Excessive blast radius | Target-count ceilings, resource-UID binding, dependency preview, policy denial, and destructive/broad actions disabled initially. |
| Confirmation fatigue | Risk-based prompts, stable concise review, no remembered production consent, and no repeated automatic retries. |
| GitOps or change-policy bypass | Explicit owner/window state, fail-closed production policy, exception receipt, and post-action reconciliation verification. |
| Malicious organization pack | Signed/versioned declarative schema, exact digest, disabled installation, capability-free data, rollback, and uninstall. |
| Credential elevation abuse | External JIT systems retain custody and approval; Automexia observes public state only. |
| Memory, queue, watch, or storage exhaustion | Numeric ceilings, fair cancellation, parked watchers, no raw streams, bounded journals, and lifecycle tests. |
| Misleading comparison or causal claim | Visible cohort/vantage/window, normalized fields, coverage and contradiction; correlation and bounded impact projection never become proof. |
| Debug or tunnel privilege expansion | Loopback default, approved immutable debug images, exact profiles/capabilities, one-use grants, external admission/authorization, process-tree/listener cleanup, and no inherited consent. |
| Model hallucination or prompt injection | No LLM in the initial path; any later small local ranker can reorder only already-valid candidates and cannot create, approve, or execute actions. |

## Lightweight intelligence strategy

The initial system is deterministic:

- typed command grammars;
- resource ownership and dependency graphs;
- health, rollout, event, telemetry, GitOps, permission, and policy predicates;
- reviewed service/runbook metadata;
- hard safety gates; and
- explainable lexicographic ranking.

An optional small local model may be evaluated only in `PO8` as a secondary
tie-breaker over candidates that already passed deterministic validation. It
receives bounded redacted features, runs off hot paths, has a strict CPU/memory/
time budget, and cannot create text, commands, targets, capabilities, approvals,
or execution requests. Deterministic ordering remains the fallback and the model
must be removable without changing safety gates.

LLM planning stays in the separate optional
[LLM Orchestration extension](LLM-ORCHESTRATION-EXTENSION.md). DevOps/SRE,
Kubernetes, cloud, GitOps, and video extensions do not depend on it.

## Delivery phases

| Phase | Outcome | Prerequisites and exit boundary |
|---|---|---|
| PO0 | Decision, research, threats, provisional limits, owner map, testing plan, and nonactivation policy | Accept or supersede ADR 0034; freeze a versioned machine contract and mutation-tested checker; approve exact owners, capabilities, and source dependencies. No runtime. |
| PO1 | Read-only production passport, pane lock, context diff, and explicit refresh | D2/D6 current-context evidence; no provider mutation or execution; route/freshness/accessibility/native/resource evidence. |
| PO2 | Bounded provider evidence; change/ownership/drift; typed resource and scheduling explanation; healthy cohort/revision comparison; read-only network path; optional SLO summaries; dependency graph and bounded impact projection | Per-feature and per-adapter capability/privacy review; Kubernetes list/watch lifecycle; correlation never silently becomes causation; no active probe, raw log/time-series cache, or disk evidence cache by default. |
| PO3 | Situation-aware diagnostics and completion ranking, including the rollout example | CP5 activation gates; deterministic candidates, reasons/refusal, insert-only behavior, CP1 fallback, stale/cancel/ranking/resource evidence. |
| PO4 | Impact preflight, policy, GitOps, permission, JIT, change-window, and confirmation composition | Accepted organization schema; current authorization checks; no execution; production fail-closed tests. |
| PO5 | Incident Mode, hypotheses, bounded timeline, optional live log fan-in, trusted/approximate time navigation, content-minimized session journal and reviewed handoff/export | Privacy/retention approval, exact stream/storage cleanup, route/workspace isolation, clock/gap semantics, accessibility and long-session evidence. |
| PO6 | Reviewed execute-observe-stabilize-verify-recover composition plus separately gated port-forward, active-probe, and safe-debug sessions | D3 and relevant provider capability activated independently; exact one-use grant, no autonomous chaining; native process/listener/provider/rollback/resource/release evidence per operation type. |
| PO7 | Signed organization runbook/policy packs and one-step cross-tool guided workflows | D7 distribution/capability decisions; declarative data only; provenance, rollback, disable/uninstall, malicious-pack, and policy-conflict evidence. |
| PO8 | Additional adapters, read-only multi-region/cluster comparison, optional small local tie-breaker, and release maturation | Explicit service-equivalence mapping and independent environment authorization; adapter-specific ADRs/evidence; deterministic fallback; three-platform native UX/accessibility/package evidence and controlled provider fixtures. |

The preferred release order is after CP5 and the relevant D6 read-only provider
foundations, with PO1-PO4 before Automation Studio needs production composition.
PO6 cannot precede D3 activation. Video editing and the optional LLM
Orchestration extension do not block this track and do not become its
dependencies.

Every phase must also satisfy its user-visible outcome and feature-by-feature
implementation checklist in the
[UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md).

## Acceptance and testing

The full scenario, oracle, security, concurrency, persistence, resource,
visual/accessibility, provider, native, rollback, and release ladder is in
[Situation-Aware Production Operations testing](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md).

No phase moves to fully implemented from documentation, mocks, test names,
cross-compilation, or a candidate count. Each phase needs current source owner,
contract tests, independent side-effect oracles, resource evidence, affected
native platform evidence, exact docs, and the release prerequisites declared by
that phase. Missing accounts, clusters, providers, hardware, signing, assistive
technology, approvals, or controlled runners remain explicit external gates.

## Primary-source baseline reviewed on 2026-08-25

- Kubernetes ownership must use
  [`ownerReferences`](https://kubernetes.io/docs/concepts/overview/working-with-objects/owners-dependents/),
  including UID and namespace rules.
- Kubernetes documents list/watch, `resourceVersion`, and bookmark behavior in
  [API concepts](https://kubernetes.io/docs/reference/using-api/api-concepts/).
- Kubernetes documents [Events](https://kubernetes.io/docs/reference/kubernetes-api/events/)
  as limited-retention, best-effort supplemental data.
- Kubernetes [Service](https://kubernetes.io/docs/tasks/debug/debug-application/debug-service/)
  and [DNS](https://kubernetes.io/docs/tasks/administer-cluster/dns-debugging-resolution/)
  troubleshooting establish a layered Service/DNS/EndpointSlice/backend path;
  Automexia still has to expose probe vantage point and unknown layers.
- Kubernetes [Server-Side Apply](https://kubernetes.io/docs/reference/using-api/server-side-apply/)
  records field-manager ownership in `managedFields`; it does not identify a
  human actor by itself.
- Kubernetes [ephemeral containers](https://kubernetes.io/docs/concepts/workloads/pods/ephemeral-containers/)
  cannot be changed or removed after addition, and
  [`kubectl debug`](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_debug/)
  also supports copied-Pod and node-debug strategies with different effects.
- Current-user authorization checks use
  [`SelfSubjectAccessReview`](https://kubernetes.io/docs/reference/access-authn-authz/authorization/).
- Rollout safety must account for Deployment
  [`maxUnavailable` and `maxSurge`](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/),
  while [Pod disruption budgets do not limit workload rolling upgrades](https://kubernetes.io/docs/concepts/workloads/pods/disruptions/).
- `CrashLoopBackOff` has several possible causes; the
  [Pod lifecycle guidance](https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/)
  requires diagnosis instead of assuming restart is the fix.
- [`kubectl diff`](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_diff/)
  is a real server-aware preview for applicable declarative changes, but it is
  not a universal dry run for operational commands.
- Kubernetes
  [plugins cannot extend existing command paths](https://kubernetes.io/docs/tasks/extend-kubectl/kubectl-plugins/),
  which supports integrating this feature through Automexia's CP5 editor bridge
  rather than pretending it is a `kubectl rollout` plugin.
- Cobra supports
  [dynamic native completions](https://github.com/spf13/cobra/blob/main/site/content/completions/_index.md),
  but they remain a syntax/source adapter rather than Automexia's policy and
  situation engine.
- Argo CD documents
  [automated self-heal](https://argo-cd.readthedocs.io/en/stable/user-guide/auto_sync/)
  and [sync windows](https://argo-cd.readthedocs.io/en/stable/user-guide/sync_windows/),
  so direct changes must account for reconciliation and change windows.
- Argo CD's [`app diff`](https://argo-cd.readthedocs.io/en/stable/user-guide/commands/argocd_app_diff/)
  and [diff customization](https://argo-cd.readthedocs.io/en/stable/user-guide/diffing/)
  plus Flux [Helm drift detection](https://fluxcd.io/flux/components/helm/helmreleases/#drift-detection)
  are provider-native desired/live and ignore-rule authorities; Automexia does
  not recreate their reconciliation semantics.
- AWS recommends
  [temporary elevated access](https://docs.aws.amazon.com/singlesignon/latest/userguide/temporary-elevated-access.html)
  for sensitive production operations, and Microsoft Entra PIM provides
  [just-in-time activation](https://learn.microsoft.com/en-us/entra/id-governance/privileged-identity-management/pim-getting-started).
- OpenTelemetry
  [semantic conventions](https://opentelemetry.io/docs/concepts/semantic-conventions/)
  and [resources](https://opentelemetry.io/docs/concepts/resources/) provide a
  useful normalization baseline without making a telemetry backend mandatory.
- Google SRE's [multiwindow, multi-burn-rate guidance](https://sre.google/workbook/alerting-on-slos/)
  supports presenting existing reviewed SLO burn evidence while retaining
  explicit request-volume and low-traffic limitations.

These sources constrain the design; they do not prove Automexia implements the
feature or that every provider behaves identically.

## Related authorities

- [Product vision](PRODUCT-VISION.md)
- [PO0 contracts and 2026 implementation decisions](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md)
- [UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md)
- [Architecture](ARCHITECTURE.md)
- [Command productivity](COMMAND-PRODUCTIVITY.md)
- [Terminal-first operations](TERMINAL-FIRST-OPERATIONS.md)
- [SSH, DevOps, and multi-cloud architecture](SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md)
- [Semantic Diagnostic Navigator](SEMANTIC-DIAGNOSTIC-NAVIGATOR.md)
- [Build, wrap, and adopt architecture](BUILD-WRAP-ADOPT-ARCHITECTURE.md)
- [Roadmap](ROADMAP.md)
- [Phase implementation audit](PHASE-IMPLEMENTATION-AUDIT.md)
