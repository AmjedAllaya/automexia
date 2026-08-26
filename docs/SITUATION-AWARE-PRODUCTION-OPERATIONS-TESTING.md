# Situation-Aware Production Operations Testing

## Status

This is the future assurance contract for `PO0-PO8`. `PO0` is documentation and
research only; `PO1-PO8` are not implemented. The commands, fixtures, crates,
native results, provider accounts, screenshots, accessibility results, and
release evidence described below do not exist unless another current repository
authority identifies them explicitly. The proposed planning contract, semantic
checker, mutation suite, and nonactivation source scan are present; they are
PO0 design evidence, not runtime or provider evidence.

The product specification is
[Situation-Aware Production Operations](SITUATION-AWARE-PRODUCTION-OPERATIONS.md).
The exact interaction, responsive-layout, wording, focus, and implementation
contract is the
[Production Operations UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md).
Exact record, freshness, rule, provider, policy, lifecycle, setting, limit and
traceability requirements are owned by the
[PO0 contracts and 2026 implementation decisions](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md).
Proposed [ADR 0034](adr/0034-situation-aware-production-operations.md) owns the
durable boundary if accepted.

## Assurance rule

A passing unit test or mocked provider is not evidence that Automexia selected a
safe production action. Every high-risk claim needs an independent oracle outside
the ranking and execution path. A phase remains partial or external until all of
its applicable exit criteria have current evidence on the exact revision and
artifact being assessed.

The proposed `PO0` machine contract now names:

- the affected feature-matrix and reinforcement entries;
- exact source, fixture, checker, fuzz, benchmark, native, visual, accessibility,
  security, provider, and release evidence owners;
- frozen resource and protocol ceilings;
- controlled provider/account/cluster/runbook fixtures;
- forbidden side effects and independent oracles; and
- the exact phase status that each evidence set can support.

Its current non-runtime checks are:

```text
python tools/ci/check_production_operations_po0.py
python tools/ci/test_production_operations_po0.py
```

They validate strict JSON and duplicate-key rejection, the canonical digest,
nonactivation, exact safety/freshness/resource values, provider profiles,
traceability and high-risk mutations. They do not replace a failing real-path
test, controlled provider fixture, native UI/accessibility result, dependency
review, benchmark, soak, protected approval, or release artifact evidence.

## UX acceptance contract

“Simple and efficient” is a tested product contract, not a visual preference.
Every phase must exercise the real controller, projection, input, renderer, CP5
bridge, focus, and accessibility paths that it changes. A static mock-up or a
pure-model test cannot satisfy this contract by itself.

| Surface | Efficiency contract | Clarity and safety contract |
|---|---|---|
| Environment passport | The persistent strip answers “where, who, how fresh, and is this production?” without opening another surface. Compact and minimal layouts preserve production, target, identity, and freshness before optional fields. | Text and semantics, not color alone, identify production, stale, locked, changed, unknown, and denied states. The strip never captures typing focus. |
| Situation list | The common read-only path is prefix → explicit completion request → arrows → Tab/Right Arrow insertion. At most five current-situation candidates are shown initially; ordinary CP5 results remain reachable. | Every row exposes action, exact target, one plain reason, freshness, and a visible read-only/review-required/refused state. No hidden mutation, implicit Enter, or opaque score is allowed. |
| Candidate detail | One explicit detail action reveals supporting, contradicting, and missing evidence without changing selection or the shell buffer. Closing returns focus and position exactly. | Correlation is labelled as correlation, unknown remains unknown, and evidence quality exposes source authority, freshness, coverage and contradiction instead of an unexplained score. Secrets and raw provider payloads never appear. |
| Preflight | A mutation candidate opens one review that groups target, impact, authority, policy/GitOps, verification, and recovery in a fixed reading order. Cancel is always available and initially safe. | Before PO6, the only positive outcome is reviewed insertion. After PO6, managed execution is a separate, explicitly labelled action with final revalidation. Manual shell execution is never presented as governed by Automexia. |
| Operation and diagnostic-session monitor | One nonmodal surface shows exactly one managed operation as preparing, running, observing, stabilizing, verifying, succeeded, failed, cancelled, uncertain, or recovery-available. A port forward, probe or debug session uses the same surface with its exact endpoint/vantage/image/profile, lifetime and stop state. The terminal remains usable. | State, target, elapsed time, before-state, last verified observation, cancellation/stop availability, cleanup state and safe next action are visible without reading raw logs. No second mutation starts automatically and no diagnostic grant becomes general mutation authority. |
| Incident workspace | One deliberate command opens objective, environment, facts, hypotheses, contradictions, missing evidence, timeline and next safe action. Optional source-separated live logs are pausable, bounded and visibly memory-only. Pinning or journaling is opt-in and never required for normal completion. | Trusted versus approximate time links, log gaps, session-only versus persisted modes, and Error Navigation handoff are obvious. Handoff/export always previews included and redacted fields plus destination. Exit restores the previous workspace and focus. |

For each moderated or instrumented task, record the starting surface, viewport,
input method, action sequence, focus transitions, completion time, errors,
backtracks, cancellations, and final external state. The participant must be able
to identify the current environment, exact target, recommendation reason, risk,
freshness, proposed effect, and safe exit without consulting internal
documentation. Tests must also prove that refreshes never steal focus, reorder a
selection underneath the user, mutate the editor, or publish stale results.

The three ownership paths are tested and worded independently:

1. **Native shell insertion:** CP5 replaces only the authenticated editor span;
   the shell owns any later Enter and Automexia offers no execution guarantee.
2. **Reviewed insertion:** preflight describes the exact text being inserted,
   but the shell still owns later execution and external state.
3. **Managed operation:** available only after PO6 activation; Automexia owns
   final revalidation, exact structured execution, observation, stabilization,
   verification, receipt, cancellation, and recovery for that one action. A
   managed diagnostic session uses this ownership path with its own one-use
   session grant, lifetime and cleanup; it grants no general mutation authority.

Usability evidence must use predefined participant criteria, realistic tasks,
success/error definitions, and published sample size before observation begins.
Report completion and error distributions plus recurring confusion; do not turn
a convenience demonstration or average alone into a broad usability claim. A
phase remains partial when keyboard, screen-reader, narrow-layout, high-scale,
offline, stale, refused, or recovery journeys have not been exercised.

## Required evidence layers

| Layer | Required evidence |
|---|---|
| Pure model | Boundary tables for every typed record, state transition, digest, expiry, risk class, refusal, ranking dimension, and limit. |
| Property and fuzz | Hostile provider data, command buffers, Unicode/control/bidi, schemas, graphs, policy packs, protocol frames, and parser limits. |
| Integration/model | Route/context/generation isolation, provider composition, cancellation, watch recovery, policy conflicts, GitOps changes, JIT expiry, execute-observe-stabilize-verify-recover transitions, and diagnostic-session cleanup. |
| Independent oracle | Exact editor bytes, exact argv, resource UIDs, provider authorization results, storage trees, process trees, network attempts, side-effect absence, and final external state. |
| Native end to end | Real supported shells, CP5 bridge, provider CLIs/APIs, disposable clusters/accounts, credential plugins, GitOps controllers, and packaged binaries on each claimed platform. |
| Visual/accessibility | Renderer-neutral hierarchy and geometry, exact controlled pixels, keyboard/focus/IME behavior, accessible roles/state/live announcements, themes, scale, and native screen readers. |
| Performance/resource | Typing latency, explicit completion latency, queries, allocations, memory, handles, threads, children, watches, cache/storage growth, long-session stability, cancellation, and final cleanup. |
| Recovery/release | Offline, stale, corrupt, rollback, disable, logout, revoke, uninstall, package identity, signing, provider cleanup, and external approval evidence. |
| Checker mutation | Removal, weakening, path substitution, fabricated evidence, threshold relaxation, stale commit, reordered gates, and false activation must fail closed. |

## Scenario inventory

Every supported scenario includes zero, one, boundary-minus-one, boundary,
boundary-plus-one, large, maximum, over-limit, repeated, fragmented, malformed,
cancelled, stale, expired, replaced, revoked, offline, timed-out, concurrent, and
shutdown variants where meaningful.

### Context passport and lock

Test:

- local/development/staging/production and unknown classifications;
- missing/partial/full provider, account, region, cluster, namespace, identity,
  role, expiry, GitOps, incident, and freshness fields;
- identical and changed account, project, tenant, subscription, region, zone,
  cluster, context, namespace, identity, role, and GitOps owner;
- pane, tab, split, cloned-session, restored-workspace, and window isolation;
- context change during completion, preflight, approval, execution, monitoring,
  recovery, logout, revocation, and shutdown;
- lock adopt, reject, expire, clear, route close, crash, restart, disable, and
  uninstall;
- production meaning conveyed through text/icon/state without depending on color; and
- comfortable, compact, minimal, and deliberately unavailable/too-small
  projections, including the exact field-removal order and a route to the full
  passport that never obstructs the shell.

Independent oracles compare the exact provider/public identity snapshot, route
ID, context revision, UI semantics, and absence of cross-pane publication.

### Kubernetes situation matrix

At minimum, controlled clusters and deterministic replays cover:

| Situation | Expected candidate behavior |
|---|---|
| Healthy stable Deployment | No incident-driven restart priority. Ordinary syntax completion remains. |
| One transient stale process with otherwise healthy dependency evidence | Restart may be offered only when disclosed evidence quality supports it and verification is defined. |
| `CrashLoopBackOff` caused by application exit | Diagnosis first unless a reviewed runbook provides stronger restart evidence. |
| Missing ConfigMap/Secret reference | Prefer configuration diagnosis; restart is not the preferred fix. |
| `ImagePullBackOff` or invalid image | Prefer image/registry diagnosis; withhold restart priority. |
| Unschedulable Pods, quota, affinity, taint, or capacity failure | Prefer scheduler/capacity diagnosis; restart must not hide the cause. |
| OOM kill or resource pressure | Show resource evidence and policy-safe diagnostics; no blind restart loop. |
| Failing liveness/startup/readiness probe | Distinguish probe configuration, startup duration, and real application failure. |
| New bad rollout correlated with all unavailable replicas | `status`, `history`, or `undo` may outrank restart if GitOps/policy allows. |
| Active healthy rollout | Prefer `status`; avoid an unnecessary second restart. |
| Stalled rollout | Show progress deadline, events, strategy, and safe diagnosis. |
| StatefulSet or DaemonSet | Use kind-specific actions and ordered/availability semantics; never emit Deployment-only arguments. |
| Bare Pod, Job, CronJob, operator/custom controller | Preserve the actual owner chain; refuse unsupported remediation. |
| Name collision or replaced resource UID | Reject stale candidate rather than retarget by name. |
| Namespace or cluster changed | Invalidate all old candidates and require context adoption. |
| RBAC allowed/denied/no-opinion/error | Match current `SelfSubjectAccessReview`; deny mutation on unavailable/ambiguous production authority. |
| GitOps auto-sync/self-heal | Explain likely reversion and prefer source/sync workflow. |
| Deny/allow sync window | Match the GitOps authority and explicit override policy. |
| `maxUnavailable`/`maxSurge`, PDB, replicas, dependencies | Show correct impact data and do not claim PDB constrains a workload rolling upgrade. |
| Watch compaction, disconnect, bookmark, relist, throttling | Preserve last-known-good as stale, recover without gaps, and never publish stale work as current. |

Owner resolution is checked against exact `ownerReferences` kind/name/UID chains,
not the production implementation's own helper result. Resource status after a
real action is checked directly from the disposable cluster and, when applicable,
the GitOps controller and observability backend.

### Evidence quality, change, ownership, and drift

Boundary tables cover every safety-relevant knowledge state: known, unknown with
reason, unavailable, unsupported, stale, redacted, and not applicable. Evidence
quality dimensions cover source authority/provenance, freshness, agreement,
completeness, and scope/window coverage independently. Tests prove that a high
value on one dimension cannot hide a failed hard gate or an unknown required
fact.

Change fixtures include rollout revision, immutable and mutable image identity,
configuration generation, scaling, Git/GitOps sync, node/provider change,
policy, CI/CD, first failure, and SLO transition. Vary source timezone, timestamp
precision, clock skew, delayed delivery, duplicate/missing Events, mutable tags,
out-of-order observations, truncated history, unavailable adapters, and
contradictory sources. Required wording says `before`, `after`, or
`correlated`; deletion/mutation of a causal-language guard must fail the
checker.

Field-ownership fixtures cover zero/one/shared managers, subresources, manager
change, forced ownership transfer, missing/oversize/malformed managed fields,
GitOps and HPA/controller ownership, audit actor present/absent, and Secret-bearing
paths. Independent oracles compare Kubernetes field sets and provider-native
GitOps desired/live, ignore, self-heal, sync and drift state. A manager name must
never be reported as a human actor without the separate audit evidence.

### Resource explanation and scheduling

Each supported kind has a table of observed facts, allowed assessments,
contradictions, unknowns, unsupported cases, and safe next diagnostics. At
minimum cover:

- Pods with healthy, crash loop, OOM, exit 137 without OOM reason, image pull,
  probe, init, terminating, replaced UID, missing/stale status, and node-pressure
  combinations;
- Deployment/StatefulSet/DaemonSet/Job/CronJob progress, generation, condition,
  ownership and kind-specific semantics;
- Service/EndpointSlice selector, port, readiness, serving/terminating,
  replacement and missing-backend cases;
- Node and PVC readiness, pressure, binding, permission, stale and unsupported
  states; and
- Pending Pods with resources, taints/tolerations, affinity, topology, PVC,
  quota, scheduler message, autoscaler, custom scheduler/plugin, and incomplete
  evidence.

Independent oracles compare exact API objects and controlled outcomes rather than
the explainer's own helper. Mutation tests prove that `CrashLoopBackOff`, exit
`137`, `Pending`, an Event, or one condition cannot become root cause alone.
The scheduling view must not claim an eligible or closest node unless an
independent provider-native result supplies it.

### Healthy cohort, revision, and environment comparison

Test cohort selection by exact owner UID, workload role, environment, revision,
readiness and observation window. Include zero peers, one peer, several peers,
singleton workloads, mixed/old revisions, terminating/unready peers, replaced
owners, duplicate names, stale members, asymmetric permissions, over-limit
cohorts, and explicit operator-selected controls.

Normalization fixtures prove that expected identity fields do not dominate while
image digest, safe configuration/Secret references, generation, limits, service
account, security context, node/zone, volumes, endpoints, restarts, conditions,
and runtime differences remain visible. Secret values and raw manifests must
never be read or persisted.

Revision/canary fixtures vary request volume, exposure duration, traffic split,
sampling, time window, error ratio, latency and restart count. Cross-region/
cluster fixtures require a reviewed service-equivalence mapping and independent
authorization/freshness per environment. The comparison cannot create a batch
mutation, merge authority, or report correlation as rollback proof.

### Network Path Diagnosis

Read-only matrices cover malformed/ambiguous destinations; local versus cluster
DNS; namespace search behavior; Service selectors and named/numeric ports;
EndpointSlice address/ready/serving/terminating/zone state; backend readiness;
NetworkPolicy ingress/egress/default-deny; ingress/gateway/mesh support; and
unknown CNI, node, cloud firewall, routing, proxy, TLS and protocol layers.

Every result asserts exact source and destination, vantage point, protocol,
freshness and one of passed/failed/unknown/not-tested/unsupported. Configuration
existence cannot satisfy an active-connectivity oracle. Local-host, source-Pod,
diagnostic-Pod, node and external probes are intentionally non-equivalent.

Controlled-probe tests inspect exact executable/image, argv/request, target,
traffic/deadline ceiling, admission, authorization, network attempts,
cancellation, temporary resources, process tree and final cleanup. Opening or
refreshing the read-only view must generate zero probe traffic.

### SLO, logs, and incident time

SLO fixtures cover valid/missing/multiple mappings, objectives and windows,
request volume, low traffic, counter reset, partial/late/stale series summaries,
backend failure, conflicting SLOs and over-limit queries. Automexia consumes
reviewed recording-rule results; tests forbid arbitrary unreviewed query
execution, time-series persistence and using impact to bypass a safety gate.

Live-log fan-in covers zero/one/maximum/over-limit sources, missing and skewed
timestamps, multiline and binary/invalid UTF-8 records, stdout/stderr, restarts,
duplicates, disconnect/reconnect, out-of-order arrival, pause/resume, source and
error filters, backpressure, byte/record limits, visible drops/gaps, route close,
Incident Mode exit, disable, uninstall and shutdown. Oracles preserve source-
local order and source identity; merged timestamp order is never claimed complete.
Session-only operation creates no disk data.

Time-navigation fixtures use trusted structured timestamps, diagnostic markers,
prompt/result anchors, approximate closest-known output, timezone/clock skew,
reflow, overwrite, eviction and missing timestamps. Exact navigation is allowed
only when the underlying owner proves the anchor; otherwise the wording and
accessibility state must say approximate.

### Candidate generation and ranking

Tests prove that:

- unsupported, unauthorized, stale, policy-denied, GitOps-conflicting,
  over-broad, target-replaced, and high-contradiction candidates are removed by
  hard gates before ranking;
- deterministic candidates are identical for identical canonical input;
- safer/reversible diagnosis precedes mutation when causal evidence is equal;
- causal evidence outranks raw alert or restart count;
- reviewed service criticality affects order only through an identified source;
- missing business criticality stays unknown;
- increasing evidence freshness or strength cannot lower a candidate except
  where another explicit safety gate changes;
- adding contradictory evidence cannot improve evidence quality or move a
  candidate ahead unless another documented dimension changes;
- ties resolve through documented stable keys;
- every candidate explains supporting, contradicting, missing, and ranking
  evidence without exposing secrets;
- the initial current-situation group contains at most five two-line rows, keeps
  ordinary CP5 results reachable, and preserves the selected stable identity
  across compatible refreshes;
- progressive detail opens only on request, closes to the exact prior focus and
  scroll position, and never changes the editor; and
- each state exposes one primary action plus a safe exit using plain labels for
  freshness, uncertainty, refusal, review requirement, and managed execution.

Metamorphic tests permute input order, duplicate observations, change unrelated
resources, and vary display labels while requiring the same target/action result.
Differential fixtures compare the ranker with a simple independent reference
model.

### Command and editor boundary

For PowerShell 7/5.1, CMD, Bash, Zsh, Fish, Nushell, WSL, quotes, selections,
mid-token edits, multiline buffers, Unicode, combining characters, IME, pasted
text, aliases, functions, and tool-native completion:

- capture exact pre/post editor bytes and cursor span;
- assert replacement only, no Enter, no duplicate suggestion surface, no shell
  evaluation, and no terminal output corruption;
- verify CP1 remains usable before, during, and after provider failure;
- reject control/bidi payloads, invalid UTF-8, untrusted annotations, oversize
  frames, stale generations, replay, endpoint confusion, and helper replacement;
  and
- prove no filesystem, network, provider, credential, policy, or process work is
  triggered by ordinary keystrokes.

Run separate real-path cases for native shell insertion, reviewed insertion,
and managed execution. Before PO6, assert that no `Execute reviewed action`
control or equivalent broker route exists. In native and reviewed insertion,
assert that a later Enter is owned by the shell and that Automexia makes no
observation, policy-enforcement, or result guarantee. In managed execution,
assert final revalidation and exact broker ownership. No state may silently fall
back from managed execution to shell text insertion or from a refused mutation
to an unlabelled advisory candidate.

### Preflight, policy, GitOps, and JIT

Boundary tables cover exact argv/target/UID, target count, risk, production state,
permissions, credential expiry, JIT eligible/active/expired/denied, approvals,
ticket/incident requirements, change windows, GitOps ownership, drift,
self-heal, sync windows, diff/plan availability, rollback, and no-rollback states.

Forbidden-side-effect oracles assert that preflight cannot:

- mutate provider, cluster, GitOps, shell, or global CLI state;
- approve or activate its own role;
- read or persist a credential value;
- turn display text into argv;
- bypass admission, RBAC/IAM, organization policy, or an external denial; or
- report an impact estimate as a real dry run.

### Incident Mode and journal

Test incident create/join/restore/handoff/exit, route and workspace isolation,
severity and objective changes, evidence timeline ordering, policy changes,
multiple operators, stale/contradictory evidence, export review, crash recovery,
retention, disable, uninstall, and disk-full/read-only/corrupt/interrupted writes.

Also test hypothesis creation/weakening/replacement, supporting/contradicting/
missing evidence, coverage-qualified negative evidence, trusted/approximate time
anchors, bounded live-log references, unresolved questions, and handoff import/
review. A handoff cannot turn a hypothesis, old evidence, or approximate time
into a current verified fact.

Compare before/after storage trees and digests. Receipts must exclude raw logs,
metrics, terminal text, output, credentials, tokens, secret-bearing environment,
raw provider responses, and arbitrary sensitive arguments. Session-only mode
must create no journal file or directory.

### Execute, observe, stabilize, verify, recover

Use a controlled scheduler and fake clock before real providers. Exercise every
state transition, cancellation point, timeout, child failure, provider loss,
context change, policy revocation, stale success, partial success, monitor loss,
rollback unavailable, recovery success/failure, repeated invocation, route close,
and shutdown.

Independent oracles inspect exact executable/argv/environment, network attempts,
external resource state, process tree, handles, children, transient files,
receipts, and absence of a second automatic mutation. No arbitrary sleeps or
retry-until-pass results count as deterministic evidence.

Verification tests distinguish observing, stabilizing, verified, failed,
cancelled, and uncertain. Exercise missing/late samples, provider disagreement,
improvement followed by regression, oscillation, new revision, GitOps reversion,
deadline boundaries, and fake-clock stabilization windows. Exit code zero never
satisfies a recovery predicate by itself.

### Managed diagnostic sessions

Port-forward tests cover loopback default, requested/random local ports,
collision, exact Service/Pod target and UID, backend replacement, Pod restart,
credential/context change, owner-route close, detach refused by default and
accepted only under a separately reviewed owner, follow off/on,
production follow refusal, non-loopback refusal, listener visibility, child-tree
termination, reconnect ceiling, application crash/restart, disable, uninstall,
and shutdown. No tunnel may inherit a grant from another pane or silently switch
environment.

Probe tests cover every supported vantage/protocol combination and prove that
unsupported or unavailable vantage points remain untested. Enforce exact
tool/image identity, traffic/byte/time ceilings, one-use grant, authorization,
admission, no retry, cancellation, temporary-resource deletion, network-attempt
oracles, and no probe from read-only view/open/refresh.

Debug-session tests distinguish ephemeral container, copied Pod and node debug.
Cover approved/unapproved/mutable images, digest replacement, profiles,
capabilities, process namespace, service account, configuration/Secret
references, admission denial, quotas, cleanup success/failure/uncertainty,
orphan recovery and external deletion. Ephemeral-container mode must explain
that it cannot be changed or removed; copied-Pod mode must not claim that
production dependencies are isolated. Node debug and privileged profiles remain
absent until separately accepted.

### Organization packs and workflows

Test valid, invalid, old/new version, unknown field, duplicate key, canonical
digest, wrong publisher, revoked key, expired pack, traversal, link, ZIP bomb,
oversize, deep graph, cycle, conflicting policy, unsupported action, hidden
capability, rollback, downgrade, disable, uninstall, and compromised update.

The verifier must prove that declarative pack content cannot create a process,
network call, credential read, shell string, executable callback, capability, or
execution grant. A one-step workflow cannot silently advance to a second
mutation.

### Provider and cross-tool evidence

Each claimed adapter has its own disposable account/environment and native
matrix. At minimum:

- supported CLI/API version and exact executable identity;
- public identity and environment resolution;
- credential plugin/agent/reference lifecycle without credential disclosure;
- read/query/watch scope, deadlines, quotas, retry, throttling, offline, and
  cancellation;
- source-to-neutral mapping and independent provider response comparison;
- action permission and exact request comparison;
- disable, logout, revoke, uninstall, transient cleanup, and no global mutation;
  and
- package, license, advisory, provenance, native architecture, startup, binary,
  memory, storage, and update evidence for every adopted dependency.

Mocks establish deterministic contracts but never replace controlled AWS, Azure,
Google Cloud, Kubernetes/OpenShift, GitOps, observability, or identity results
for a release claim that names those systems.

## Security and hostile-input campaigns

Seed property/fuzz corpora with every historical failure and with:

- malformed/fragmented/oversized protocol frames and partial UTF-8;
- Unicode normalization, combining, bidi, control, escape, newline, null, and
  misleading homoglyph data;
- hostile Kubernetes labels, annotations, Events, object graphs, owner cycles,
  duplicate UIDs, replaced resources, managed fields, EndpointSlices, debug
  profiles/images, log records, and watch streams;
- secret-shaped canaries in every provider field, log/event summary, runbook,
  command annotation, environment, and error;
- path traversal, links, replacement, permissions, disk-full, corruption, and
  interrupted persistence;
- graph cycles, exponential fan-out, duplicate evidence, ranking-field overflow,
  hostile cohort selection, timestamp/clock manipulation, NaN/infinity where
  relevant, and stable-order attacks; and
- cancellation, queue saturation, crash loops, provider restart, clock skew,
  expiry boundaries, shutdown, and uninstall.

Record fuzz engine, seed, corpus digest, duration, executions, sanitizer,
platform, and discovered/replayed crashes. A smoke campaign proves only that
campaign.

## Visual and accessibility matrix

Renderer-neutral tests cover hierarchy, reading order, focus, clipping, z-order,
target identity, risk, freshness, evidence quality, disabled/refusal states,
context diff, state explanation, change timeline, cohort comparison, network
layers/vantage, SLO limitations, live-log gaps, time-anchor precision, preflight,
Incident Mode, managed-session review, monitor, recovery, and live announcements.

Controlled exact-pixel fixtures cover tiny through 8K-equivalent viewports,
split panes, 100-300% scale, dark/light/high-contrast/custom themes, long and
localized identifiers, missing/large candidate groups, keyboard/pointer/IME,
focus loss/restore, modal stacking, animations on/off, and reduced motion. The
repository's zero-tolerance pixel policy applies to deterministic fixtures.

Native evidence includes keyboard-only operation and current Narrator/NVDA on
Windows, VoiceOver on macOS, and Orca on X11/Wayland for any platform release
claim. Automated accessibility trees do not replace controlled assistive-
technology results.

The six-surface hierarchy from the UX blueprint is checked explicitly: passport
is persistent but nonfocusing; situation list and detail are pane-local;
preflight is the only deliberate review interruption; monitor is nonmodal; and
Incident workspace is explicitly entered and exited. Hidden, clipped,
superseded, background-pane, stale-generation, and closed surfaces must own no
focusable node, hit target, live announcement, or input route. Refresh preserves
focus by stable identity or moves it through the documented deterministic rule.

## Performance, resource, and storage evidence

Measure the real owning paths on named hardware with repeated samples and
same-host baselines:

- ordinary keystroke and frame latency with `PO` enabled/disabled/failed;
- warm/cold explicit contextual completion p50/p95/p99;
- parsing, graph update, hard-gate, ranking, projection, and insertion cost;
- provider request count, QPS, bytes, concurrency, timeouts, retries, watch and
  relist frequency;
- comparison normalization/fan-out, live-log streams/bytes/drops, time-anchor
  lookup, SLO requests, tunnel listeners, probe traffic, and debug resources;
- CPU, allocations, resident/private memory, threads, handles, sockets, child
  processes, files, cache and journal growth;
- 1, 32, and over-limit panes; large/over-limit namespaces; disconnect/reconnect;
  rapid context switching; 24-hour incident and 30-day controlled soak; and
- final cleanup after route close, Incident Mode exit, provider revoke, helper
  crash, disable, uninstall, application shutdown, and forced low-resource
  failures.

The `PO0` provisional ceilings in the product specification and proposed machine
contract are failure thresholds, not targets to consume. A late performance fix
reruns affected correctness, security, visual, and resource gates.

## Phase exit criteria

| Phase | Minimum evidence before “fully implemented” |
|---|---|
| PO0 | Proposed numeric contract/checker/mutations and current source nonactivation scan exist. Still required: accepted/superseded ADR and exact digest, protected owner/capability/dependency review, reviewed prototypes, first failing real-path tests, independent threat review, and release-artifact nonactivation proof. |
| PO1 | Pure/context integration tests, exact route isolation, production lock/refusal UX, native shell/window/context cases, controlled pixels/accessibility, resource and cleanup evidence. |
| PO2 | Adapter contracts; evidence-quality and unknown-state tables; change/ownership/drift; resource/scheduling explainers; cohort/revision comparison; read-only network path; SLO summaries; graph/property/fuzz/list-watch models; provider-native read-only evidence, quotas, stale/offline/recovery, resources, disable/uninstall. |
| PO3 | Kubernetes and cross-tool scenario corpus, independent ranking oracle, exact editor bytes/no Enter, CP1 fallback, stale/cancel/performance, native CP5 integration and accessibility. |
| PO4 | Permission/policy/GitOps/JIT/preflight tables, false-dry-run prevention, production fail-closed, provider authority comparisons, visual/accessibility and security approval. |
| PO5 | Incident/hypothesis/journal state; negative-evidence coverage; live-log ordering/gaps/backpressure; trusted/approximate time navigation; privacy and zero-write session tests; storage fault/recovery/retention/uninstall; long-session resources, reviewed handoff/export, native collaboration/accessibility. |
| PO6 | Separately activated D3/provider gates; one-use grant; real exact-argv/provider workflows; observation/stabilization/regression; process/listener/resource cleanup; port-forward/probe/debug matrices; verification/recovery/rollback; no autonomous chaining; package and controlled release evidence. |
| PO7 | Signed malicious-pack corpus, declarative no-capability proof, policy conflict, cross-tool step boundary, atomic rollback/disable/uninstall, provenance and native package evidence. |
| PO8 | Per-adapter controlled matrices, explicit service-equivalence and independent authorization for read-only multi-region/cluster comparison, optional local-ranker differential/safety/resource proof, deterministic fallback, three-platform release UX/accessibility/packages and soak. |

Every row also requires the phase-specific surface, journey, wording,
responsive-state, keyboard/focus, accessibility, visual, perceived-performance,
and moderated-usability evidence defined by the
[UX and implementation blueprint](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md).

Unavailable credentials, accounts, clusters, GitOps installations, observability
backends, organization policies, hardware, signing, protected reviewers, native
runners, or assistive technologies stay external. They are never converted to a
passing status by documentation or simulation.
