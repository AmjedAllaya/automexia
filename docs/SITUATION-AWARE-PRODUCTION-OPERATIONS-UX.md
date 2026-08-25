# Situation-Aware Production Operations UX and Implementation Blueprint

## Status and authority

This document is the canonical user-experience and implementation blueprint for
the planned `PO0-PO8` production-operations track. It extends the
[product specification](SITUATION-AWARE-PRODUCTION-OPERATIONS.md), the
[exact proposed PO0 contracts](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md),
the
[testing contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md), and
[proposed ADR 0034](adr/0034-situation-aware-production-operations.md).

`PO0` is documentation and research only. None of the surfaces, actions,
shortcuts, settings, adapters, investigation views, live-log controllers,
watchers, managed diagnostic sessions, journals, models, or execution paths in
this document is implemented or enabled. The proposed machine contract now
freezes names, dimensions, defaults and ceilings for review, but it is
non-activating and remains subject to protected ADR/owner acceptance.

The intended result is not a control room squeezed into a terminal. The normal
terminal stays visually quiet. Automexia adds the least information needed at
the moment it helps, keeps the native command visible, and makes every sensitive
step easy to cancel.

## Experience outcome

An operator should be able to answer five questions without leaving the active
pane:

1. **Where am I?** The account, region, cluster, namespace, identity, and
   production state are clear.
2. **What changed or is unhealthy?** Fresh evidence and important uncertainty
   are summarized without copying raw telemetry into the terminal.
3. **What can I safely do next?** A short list of native diagnostics or actions
   is ordered by evidence, safety, and reviewed service importance.
4. **What will this affect?** Exact targets, permissions, GitOps behavior,
   blast radius, stop signals, and recovery are visible before a managed action.
5. **Did it work?** Observation and stabilization end in a clear verified,
   failed, uncertain, cancelled, recovered, or recovery-available state.

The common path should require no new window and no form. More detail appears
only when the user requests it or when a blocker makes it necessary.

## Simplicity contract

Every PO surface follows these rules:

1. **Quiet by default.** Only the compact environment passport persists. No
   background popup, toast, model prompt, or activity feed competes with typing.
2. **One task, one primary action.** A view may offer several secondary links,
   but it presents at most one emphasized next action and always exposes a safe
   cancel or back action.
3. **Progressive detail.** Candidate rows show command, reason, target, risk,
   and freshness. Evidence, contradictions, policy, and recovery expand on
   request instead of filling the terminal by default.
4. **Native commands stay visible.** Users can understand, copy, and run the
   same commands outside Automexia. Product labels never replace the real
   executable and arguments.
5. **No surprise focus.** Refreshes do not steal focus, move the cursor, select
   a row, or open a modal. The user explicitly opens every interactive surface.
6. **No surprise execution.** Completion selection never presses Enter.
   Managed execution is a different, explicitly chosen flow.
7. **Uncertainty is concise.** Use plain language such as `Evidence is stale`,
   `Cause is not clear`, or `Restart is unlikely to help`; do not expose an
   unexplained numeric confidence score.
8. **Production is unmistakable.** The word `PRODUCTION`, an icon/shape, and the
   relevant environment name remain visible. Color is supporting information.
9. **Errors stay in context.** Prefer a small inline state with `Retry`, `Use
   cached`, or `Show details`. Use a blocking dialog only for an intentional
   managed mutation that needs immediate confirmation.
10. **The terminal always has an exit.** `Escape`, cancel, feature disable,
    adapter failure, and uninstall return to native terminal and CP1 behavior
    without changing the command buffer unexpectedly.

## Surface map

The feature uses six surfaces. They share one route-scoped state owner and do
not appear simultaneously unless the hierarchy below permits it.

| Surface | When visible | Purpose | Input ownership |
|---|---|---|---|
| Environment passport | Above the active command when PO context exists | Show exact current environment, identity, lock, and freshness | Noninteractive by default; explicit open/lock action |
| Situation list | Explicit completion request with current candidates | Show a short grouped list of native next steps | Existing CP5 listbox/controller |
| Candidate detail | Explicit `Review details` action | Explain evidence, contradictions, impact, and alternatives | App-owned pane detail projection; no PTY input |
| Production preflight | Explicit review of a mutating candidate | Revalidate exact target, authority, policy, effect, and recovery before reviewed insertion or a separately activated managed action | App-owned review surface; cancel-first |
| Operation monitor | Only while a PO6 managed action is active or retained | Show bounded progress, verification, failure, and recovery | App-owned nonmodal operation card |
| Incident workspace | Explicit Incident Mode | Pin incident context, evidence summaries, attempted actions, and handoff | App-owned pane/workspace state |

Live log fan-in is a bounded mode inside the Incident workspace. Network path,
comparison, state explanation, change, drift, and SLO views are modes inside
candidate detail. Port forwards, probes, and debug sessions use preflight plus
the operation monitor. They do not increase this top-level surface count or add
competing focus owners.

Visual priority is:

```text
managed confirmation or recovery decision
  > candidate detail / Incident workspace
  > situation list
  > environment passport
  > terminal grid
```

A higher surface dismisses or suspends the lower interactive surface. Hidden
surfaces receive no pointer, keyboard, wheel, IME, drop, or accessibility input.
The terminal grid remains visible whenever the viewport can contain it safely.

## Responsive density

PO follows the existing logical viewport and scale model. Exact thresholds must
be frozen with the UI implementation, but the behavior is already fixed:

| Pane condition | Experience |
|---|---|
| Comfortable | Passport shows environment plus selected scope. Situation list shows command, one-line reason, risk, and freshness. Detail may use a list-plus-detail layout. |
| Compact | Passport collapses secondary fields into a `More context` action. Situation rows keep command, target, and one short reason. Detail replaces the list and provides a clear Back action. |
| Minimal | Passport keeps `PRODUCTION`/environment and stale/locked state only. Completion becomes the existing compact CP5 hint. Details use a temporary full-pane route that preserves the editor buffer and returns focus exactly. |
| Too small for a safe review | Do not clip a confirmation. Keep the command unexecuted and say `Widen this pane to review safely`, with an accessible status announcement. |

Nothing becomes smaller merely to fit. Text may wrap or move to the next level,
but target identity, production state, risk, blockers, primary action, and cancel
remain readable. Large and 4K/8K viewports do not enlarge controls; they expose
more terminal rows and optional detail.

## Interaction contract

### Completion list

Reuse the current CP5 interaction rather than adding a second key model:

- the user's normal completion action opens native candidates and, when ready,
  a separate `Current situation` group;
- Up/Down move the active option; Home/End move to the first/last option where
  the platform mapping permits it;
- `Escape` dismisses without changing the editor buffer;
- optional `Tab` or Right Arrow performs the existing authenticated replacement
  only when that CP5 acceptance mode is enabled;
- `Enter` remains owned by the shell/editor and is never captured as a PO
  acceptance shortcut;
- pointer release over the same row selects it; hover alone never changes the
  editor buffer;
- typing, IME start, selection change, prompt completion, focus loss, modal
  opening, route change, or stale generation dismisses the list; and
- `Review details` and `Review managed action` are named, remappable app actions
  with visible buttons/command-palette entries. No new default shortcut is
  assigned until the keybinding collision audit passes.

The selected state and keyboard focus are distinct. Moving through rows may
update the side detail, but it must not insert or approve anything.

### Advisory shell path versus managed path

Automexia cannot govern a command that a user types or runs directly in their
shell. The UX must never imply otherwise.

| Path | User experience | Enforceable guarantee |
|---|---|---|
| Native/manual shell | User types or inserts a visible native command and may press Enter in the shell | Automexia can explain and warn, but cannot enforce preflight, policy, monitoring, or recovery after shell ownership resumes |
| Reviewed insertion | User opens PO4 preflight, reviews current evidence, then chooses `Insert reviewed command` | The inserted bytes and reviewed revision are exact; external state can still change before the user presses Enter |
| Managed operation | User chooses `Execute reviewed action` only after PO6/D3/provider activation | Automexia revalidates the exact action and can enforce its own approval, process, observation, stabilization, verification, timeout, receipt, and recovery contract |

An organization policy may hide or disable PO-provided mutating insertions and
offer only `Review managed action`. This does not block manually typed shell
commands; that boundary must be stated in policy and product copy.

### Focus and announcements

- Opening a list keeps editor focus and uses active-descendant semantics.
- Opening detail moves focus to its heading or Back action according to the
  renderer's platform convention and restores the prior row/editor exactly.
- Production preflight initially focuses the safe cancel/back action for production
  or destructive operations. Enter is not a default execution action for an
  irreversible or broad operation.
- Live announcements are coalesced. Announce list result count, selected target
  and risk, refresh completion, context invalidation, blockers, operation state,
  and final result. Do not read raw logs or every watch event.
- Focus indicators stay visible and unobscured by the passport, popup, footer,
  monitor, IME candidate window, or modal stack.

## PO1 — environment passport and context lock

### Environment passport: what the user sees

The passport sits directly above the command and uses the project's existing
context-strip visual language.

Comfortable example:

```text
PRODUCTION  AWS payments-prod  eu-west-1  EKS core-prod  ns/payments
identity: sre-oncall  elevation: 18m  GitOps: payments/live  fresh: 3s  LOCKED
```

Compact example:

```text
PRODUCTION  core-prod / payments  sre-oncall  fresh 3s  LOCKED
```

Unknown or stale context is explicit:

```text
PRODUCTION?  cluster unknown  evidence stale 2m  [Refresh context]
```

### Environment passport: simple interaction

- Opening the passport shows public context fields and their sources.
- `Lock this pane` binds the current route/context revision.
- If account, region, cluster, namespace, identity, GitOps owner, or incident
  changes, the passport does not silently update a locked production review. It
  shows `Context changed` and one concise before/after diff.
- The primary choice is `Use new context`; the safe choice is `Keep current
  pane locked` or `Cancel review` depending on whether the old context still
  exists.
- Locking is optional for read-only ordinary terminal work and mandatory only
  inside Automexia-managed production mutation review.

### States and copy

| State | Visible wording | Available action |
|---|---|---|
| Fresh | `Fresh 3s` | Open details or lock |
| Refreshing | `Refreshing context…` without blocking typing | Cancel refresh |
| Stale usable | `Stale 2m · read-only guidance` | Refresh |
| Partial | `Namespace known · identity unavailable` | Show missing fields |
| Changed while locked | `Context changed · review required` | Compare |
| Permission expired | `Elevation expired` | Open provider-owned access instructions |
| Adapter failed | `AWS context unavailable · terminal unaffected` | Retry or disable adapter |

### Environment passport: implementation requirements

1. Define a versioned `EnvironmentPassport` with bounded public identifiers,
   field-level source/freshness, route, session, revision, expiry, lock state,
   and deterministic digest.
2. Project existing D2/D6 cached public context into the pure operation model;
   never reread providers in the renderer.
3. Add one app-owned controller for explicit refresh, lock, diff, adopt, clear,
   cancellation, revocation, route close, and shutdown.
4. Add renderer-neutral comfortable/compact/minimal projections and semantic
   nodes for status, lock, changed fields, and actions.
5. Persist no passport by default. If later workspace restore retains public
   intent, it must revalidate before becoming current and never restore a live
   credential/elevation claim.
6. Add exact pane/tab/window/clone isolation, stale publication, rapid switch,
   provider loss, accessibility, visual, latency, memory, and cleanup tests.

## PO2 — bounded investigation evidence

### Investigation: what the user sees

PO2 has no permanent dashboard. The user sees one short state in the passport
and detail views:

```text
Evidence: 14 current · 2 stale · 1 source unavailable
Recent change: deployment/payments-api revision 48, 6m ago
Dependencies: checkout -> payments-api -> postgres
```

The detail view groups evidence into `Health`, `Recent changes`, `Ownership`,
`Comparison`, `Network path`, `User impact`, `Dependencies`, and
`Missing`. Raw log bodies and metric series open in their own trusted tool
through a stable source reference; they are not copied into the PO2 evidence
cache.

### Investigation: simple interaction

- `Refresh evidence` is explicit and cancellable.
- Default detail shows at most three decisive observations and one important
  contradiction. `Show all evidence` reveals the bounded remainder.
- Recent changes are described as correlation, not cause: `Failure began 2m
  after revision 48` rather than `revision 48 caused the failure`.
- Selecting a resource shows its exact kind/name/UID, owner chain, direct
  dependents, freshness, and source.
- Cycles, unknown ownership, or over-limit graphs show an honest partial state.
- One `Investigate` action reveals no more than five relevant entry points:
  `Explain state`, `What changed?`, `Compare with healthy`, `Diagnose
  connection`, and `Show user impact`. Unsupported or irrelevant actions are
  omitted with an accessible explanation in detail rather than disabled clutter.

### Investigation actions and views

The read-only investigation capabilities reuse candidate detail and Incident
Mode. Managed diagnostic sessions reuse preflight and the operation monitor.
They do not add permanent dashboards or fifteen command-palette entries.

#### Explain state

The first screen answers what is observed before offering an interpretation:

```text
Pod payments-api-849bd7

Observed       CrashLoopBackOff · last termination OOMKilled · restarts 17
Resources      request 256 MiB · limit 512 MiB
Controller     Deployment/payments-api · revision 185
Cohort         4 healthy · 1 affected
Assessment     Memory exhaustion is supported; node pressure is not yet checked

[Compare with healthy]  [Show memory source]  [Previous-container logs]
```

The result uses kind-specific sections. It never turns `CrashLoopBackOff`,
`Pending`, exit code `137`, an Event, or a condition into root cause by
itself. Scheduling detail groups observed scheduler reasons, requests, taints,
affinity/topology, PVC state, node summaries, and autoscaler state; it does not
invent a closest node.

#### What changed, ownership, and drift

`What changed?` opens a bounded timeline anchored to the selected resource,
diagnostic marker, or explicit incident time:

```text
14:36  Deployment revision 184 -> 185       observed by Kubernetes
14:37  New Pods became ready                 observed by Kubernetes
14:40  ConfigMap generation changed          observed by Kubernetes
14:41  First DB timeout                      approximate log timestamp
14:42  Error-budget burn crossed policy      SLO source
```

The header shows timezone, source-clock quality, window, missing sources, and
`Correlation is not proof of cause`. Opening a change reveals safe fingerprints
and source links, never a full Secret or unredacted manifest.

`Inspect ownership` lists field manager, GitOps owner, source revision, drift
classification, ignore/self-heal behavior, and freshness. It says `manager:
argocd-controller`, not `Alice changed this`, unless an authorized audit source
actually supplies the human identity.

#### Compare with healthy

The view first explains the cohort:

```text
Control cohort: 4 ready Pods · same Deployment UID · same revision · fresh 4s
Excluded: 1 terminating Pod · 1 older revision

Different                     Affected                Healthy cohort
Node condition                MemoryPressure          none observed
Readiness                     false                   true (4/4)
Image digest                  b71c...                 b71c...
Configuration references      same                    same
```

The user may deliberately switch to `Compare revisions`. Request volume,
exposure duration, traffic split, error ratio, latency, and restart summaries
stay beside the conclusion. Cross-region/cluster comparison appears only in PO8
after a reviewed service-equivalence mapping and independent authorization for
every environment.

#### Diagnose connection

The initial view is read-only and always names source and destination:

```text
Source       Pod/payment-api-7df82
Destination  Service/postgres:5432

DNS              passed      Service resolves to 10.96.81.42
Service          passed      port 5432 -> targetPort 5432
EndpointSlices   passed      3 ready endpoints
NetworkPolicy    unknown     policies apply; effective path not proven
TCP              not tested  active probe requires review
TLS              not tested
```

Selecting `Review controlled probe` enters PO6 preflight. A local-host probe
never silently substitutes for a Pod-vantage probe, and an untested layer never
receives a green check.

#### Show user impact

When a reviewed SLO mapping exists, the detail shows objective, window, remaining
budget, burn rate, request volume, error ratio, source, and freshness. If traffic
is too low or the service has no mapping, the view states that limitation. User
impact may change ordering among already-valid recommendations but never grants
authority or removes a safety gate.

### Investigation: implementation requirements

1. Define versioned `Observation`, `KnowledgeState`, `EvidenceQuality`,
   `ChangeRecord`, `FieldOwnershipRecord`, `ResourceExplanation`,
   `CohortDefinition`, `ComparisonFinding`, `NetworkPathAssessment`,
   `SloImpactSummary`, `ResourceNode`, and `ResourceEdge` contracts with source,
   provider/local time, clock quality, observed time, expiry, route/context, UID,
   redaction class, coverage, and digest.
2. Build bounded adapter mappings for Kubernetes objects/Events/rollouts,
   approved telemetry summaries, GitOps revisions/sync state, provider health,
   service catalogs, incidents, and organization metadata.
3. Resolve Kubernetes ownership by kind/name/UID owner-reference chains. Reject
   name guessing, cross-namespace illegal ownership, cycles, replaced UIDs, and
   unsupported controllers.
4. Add bounded recent-change correlation for deployments, images,
   configuration generations, Git revisions, scaling, node/provider events,
   policy changes, and adapter-specific changes. Correlation cannot become a
   causal statement without a reviewed rule and supporting evidence.
5. Add kind-specific structured explainers and scheduling findings. Keep
   observed fact, assessment, recommendation, and typed action separate.
6. Add bounded field-ownership and desired/live drift mappings that consume
   provider-native managers, diffs, ignore rules, sync, and self-heal state.
7. Add cohort selection/normalization and revision comparison with visible
   inclusion/exclusion, sample, traffic, window, and no-comparable-control states.
8. Add read-only layered network mapping with explicit source/destination,
   vantage point, per-layer result, unknowns, and a typed handoff to PO6 probes.
9. Add optional reviewed SLO mappings that return summaries, request volume,
   low-traffic state, source, and freshness rather than arbitrary time series.
10. Maintain an immutable route-scoped graph off hot paths with explicit list/
   watch or polling limits, relist/recovery, last-known-good publication, LRU
   eviction, parked watchers, and generation cancellation.
11. Return bounded summaries and stable source references only. Keep raw logs,
   time series, provider payloads, secrets, and credential data out of caches
   and persistence.
12. Test graph/cohort boundaries, field normalization, fan-out, cycles, order
   independence, duplicate/stale observations, watch gaps, source disagreement,
   clock skew, false-causal language, secret canaries, low traffic, unavailable
   vantage points, offline recovery, resource ceilings, and final cleanup.

## PO3 — situation-aware completion

### Situation-aware completion: what the user sees

The existing CP5 popup keeps native results and adds one clearly separated
group only when the user asks for completion:

```text
Current situation · prod/core-prod/payments · fresh 3s

> kubectl rollout status deployment/payments-api -n payments
  Diagnose first · rollout is still progressing                         Read only

  kubectl rollout undo deployment/payments-api -n payments
  Revision 48 matches 8 unavailable replicas                    Production change

  kubectl rollout restart deployment/notification-worker -n payments
  May clear stale connections · configuration cause not excluded       Medium

  2 sources stale · Review details
```

Each row has exactly two visual lines in comfortable density:

1. native command and target;
2. short reason plus risk/freshness state.

The selected row may expose a restrained third-line footer for `Why`, `Review`,
or `Unavailable`. Long explanations belong in candidate detail.

### Situation-aware completion: simple interaction

- Native shell candidates remain first when no current operational candidate is
  clearly useful.
- Default view shows at most five operational candidates; scrolling reveals up
  to the bounded maximum.
- Read-only diagnostics are selectable immediately.
- Production mutations show `Review required` when policy requires the managed
  path. They do not masquerade as ordinary safe completion.
- One action opens details with sections: `Why this`, `What conflicts`, `What
  could change the answer`, `Expected result`, and `Safer alternatives`.
- If evidence is insufficient, the only result may be `Collect more evidence`
  or a read-only diagnostic. An empty list says why instead of disappearing.

### Situation-aware completion: implementation requirements

1. Extend the pure model with `OperationalIntent`, `CandidateAction`, evidence
   references, contradiction/missing sets, risk, expected effect, verification,
   recovery, expiry, and stable deterministic keys.
2. Add strict command grammars and adapter rule packs. Rules may create only
   registered typed actions with exact schemas and never concatenate shell
   strings from provider text.
3. Apply route, context, UID, generation, freshness, support, capability,
   authorization, policy, GitOps, target-count, blast-radius, and contradiction
   gates before ranking.
4. Rank lexicographically by safety/reversibility, causal evidence, reviewed
   criticality, dependency order/customer impact, urgency, freshness,
   organization preference, and stable tie-breakers.
5. Reuse the CP5 request, UI, accessibility, and authenticated replacement
   owner. Add a typed source adapter; do not query providers per keystroke or
   create another popup/controller.
6. Keep candidate detail in an app-owned immutable projection. Display strings
   never become argv or replacement bytes.
7. Implement the complete Kubernetes situation corpus, including healthy,
   active/stalled/bad rollout, crash loop, missing configuration, image pull,
   scheduling, quota, storage, probes, OOM, dependencies, custom controllers,
   replaced UIDs, GitOps, and authorization states.
8. Test exact editor bytes, cursor/selection/quotes/IME, no Enter, CP1 fallback,
   stale cancellation, deterministic/metamorphic ranking, independent oracles,
   native shells, accessibility, pixels, p50/p95/p99, memory, and cleanup.

## PO4 — candidate detail and production preflight

### Production preflight: what the user sees

Candidate detail is explanatory and nonmodal. Production preflight is a deliberate
review with one clear summary:

```text
Review production action

Restart deployment/notification-worker
PRODUCTION · core-prod · payments · UID 7f…91 · evidence fresh 2s

Expected effect     Recreate 4 Pods; service remains available if 3 replicas stay ready
Main uncertainty    Configuration cause is not excluded
GitOps              Direct restart is temporary; declared state is unchanged
Permission          Allowed until 18:42
Recovery            Stop monitor; inspect new Pods; no automatic second restart

[Cancel]                                      [Insert reviewed command]
```

After PO6 activation, the primary action may be `Execute reviewed action`.
Before PO6 it cannot appear enabled.

### Production preflight: simple interaction

- Blockers appear before detail and disable the primary action.
- `Why blocked?` lists one plain-language reason per failed gate and the owning
  authority. It never suggests bypassing an external denial.
- `Impact`, `Evidence`, `Policy and access`, and `Recovery` are collapsed by
  default unless one contains a blocker.
- Exact executable and arguments are always available and copyable, but display
  text is separated from the typed action.
- Typed confirmation is reserved for broad, destructive, or organization-
  required production actions. Ordinary read-only actions never ask for it.
- Preflight expires visibly. Refresh preserves the user's place but creates a
  new digest and invalidates the previous approval.

### Production preflight: implementation requirements

1. Define `RiskAssessment` and `ImpactPreflightSnapshot` with exact executable/argv,
   target UIDs/count, dependencies, health, strategy, permission, JIT expiry,
   policy, change window, GitOps, estimate/diff provenance, uncertainty,
   success/stop/timeout/recovery, expiry, and digest.
2. Implement independent current authorization and mutable-evidence refresh.
   For Kubernetes use the supported authorization review; for each cloud or
   GitOps adapter use its approved authority.
3. Label real server diff/plan separately from an impact estimate. Never call a
   local projection a dry run.
4. Bind confirmation to one exact route, passport, target UID, action, evidence
   generation, policy revision, authority result, expiry, and use.
5. Separate advisory reviewed insertion from managed execution in API, copy,
   button label, receipt, and tests.
6. Implement renderer-neutral review state, responsive details, cancel-first
   focus, disabled/blocker semantics, accessible descriptions, and focus restore.
7. Test policy/JIT/GitOps/change-window combinations, expiry boundaries,
   replaced targets, permission loss, false dry-run language, secret redaction,
   no side effects, visual/accessibility states, and cancellation.

## PO5 — Incident Mode and journal

### Incident Mode: what the user sees

Incident Mode changes the passport, not the entire terminal theme:

```text
INCIDENT SEV-1  INC-2841  payments unavailable  PRODUCTION core-prod  role: operator
```

An optional side/bottom workspace contains five short sections:

1. `Objective` — the current recovery goal and stop condition;
2. `Now` — decisive fresh evidence and missing sources;
3. `Hypotheses` — leading, alternative, and weakened hypotheses with supporting,
   contradicting, missing, and coverage-qualified negative evidence;
4. `Tried` — reviewed actions with result and time;
5. `Next` — one selected diagnostic or managed action plus alternatives.

The timeline is a bounded summary, not a log viewer. Handoff export first shows
exactly what will be included and redacted.

### Incident Mode: simple interaction

- `Start Incident Mode` asks only for incident reference and objective; severity
  and role are optional or adapter-provided.
- Joining or switching incidents shows the exact context change before adoption.
- `Exit Incident Mode` cancels PO work and parks watches but never ends external
  provider incidents or modifies tickets automatically.
- Timeline entries with a trusted source timestamp can offer `Jump to event`.
  Approximate terminal anchors say `Closest known output` and show clock/source
  limitations rather than pretending every scrollback line is timestamped.
- `Follow related logs` opens a bounded memory-only fan-in inside the Incident
  workspace. Every row keeps a visible Pod/container source, pause/resume and
  source filters; dropped or disconnected intervals remain visible.
- `Previous diagnostic` delegates navigation to Error Navigation using a
  bounded marker reference. Incident Mode neither copies terminal history nor
  becomes a second diagnostic scanner.
- `Export handoff` previews public environment, evidence/action digests,
  facts, hypotheses, contradictions, outcomes, blockers, unresolved questions,
  and source links. Raw terminal and telemetry content stay excluded.
- The session journal needs no setup and disappears with the session. Protected
  persistence is a separate opt-in with retention and removal controls.

### Incident Mode: implementation requirements

1. Add versioned incident identity, objective, severity, role, route/workspace,
   hypothesis/negative-evidence state, evidence-summary timeline, trusted or
   approximate time anchors, operation link, handoff, and lifecycle types.
2. Keep one app-owned Incident Mode controller with explicit start/join/change/
   exit, generation cancellation, provider revocation, crash recovery, and
   shutdown behavior.
3. Define `ActionReceipt` as a content-minimized record. Allowlist public fields;
   exclude raw command output, logs, metrics, terminal text, environment values,
   credentials, tokens, provider responses, and arbitrary arguments.
4. Implement memory-only bounded session storage first. Any protected local
   persistence must use private no-follow atomic storage, schema migration,
   corruption recovery, quotas, retention, disable, export, and exact uninstall.
5. Add a separate bounded live-log view owner with per-source sequence/timestamp/
   arrival/gap state, backpressure, source filters, pause/resume, Error Navigation
   references, and memory-only cleanup. Never concatenate or persist one raw log.
6. Add renderer-neutral responsive workspace/timeline/handoff projections,
   keyboard navigation, accessible grouping/live status, and no color-only
   severity.
7. Test multiple incidents/operators/routes, hypothesis transitions, negative-
   evidence coverage, trusted/approximate navigation, source-local and cross-
   source ordering, clock skew, disconnect/gaps/drops, filters, conflicting
   evidence, stale policy, long sessions, disk faults, privacy canaries, storage
   trees/digests, focus, pixels, assistive technology, and cleanup.

## PO6 — managed execution, observation, stabilization, verification, and recovery

### Managed operations: what the user sees

Managed execution is never a generic terminal spinner. It is a nonmodal card
bound to the exact operation:

```text
Restarting deployment/notification-worker       Observing · 00:18 / 02:00
2/4 replacement Pods ready · service endpoints healthy

[Show evidence]  [Stop monitoring]  [Open recovery]
```

Final states use explicit words:

- `Verified — 4/4 Pods ready and error rate recovered`;
- `Failed — new Pods did not become ready`;
- `Uncertain — deadline reached; external state may still be changing`;
- `Cancelled — execution stopped before launch`;
- `Uncertain — monitoring stopped; external state is not verified`; and
- `Recovery available` / `No automatic recovery is defined`.

### Managed operations: simple interaction

- The user can work in other panes while the card remains attached to the
  owning route.
- `Cancel before launch`, `Stop monitoring`, `Request external abort`, and
  `Recover` are different actions with truthful semantics.
- A timeout never reports failure or rollback unless the provider proves it.
- The monitor separates `Observing` from `Stabilizing` and shows the declared
  window plus any predeclared regression signal. Missing samples become
  `Uncertain`; they are not interpolated into success.
- A failed verification offers one reviewed recovery action or read-only
  diagnostics, then returns control. It never launches another mutation.
- Closing the route asks whether to transfer observation to an explicit safe
  app-owned owner or stop observing; it never implies that stopping observation
  reverses an external change, and no Automexia-owned process/watch is orphaned.

### Managed operations: implementation requirements

1. Reuse the one app-owned D3 exact-argument/provider broker. PO creates no
   second process, network, credential, or execution path.
2. Implement a versioned operation state machine with explicit transition
   guards, route/generation owner, deadlines, cancellation, provider loss,
   shutdown, before-state summary, observation, stabilization, regression,
   result, verification, recovery offer, and receipt.
3. Revalidate exact candidate/preflight/authority/policy/target immediately
   before one-use launch. Reject rather than retarget or reuse an approval.
4. Define registered typed per-action success, failure, stop, timeout,
   stabilization, regression, and recovery predicates. Do not begin with a
   general expression language.
   Monitoring reads bounded current evidence and never evaluates raw display
   strings as control data.
5. Own and join every worker, process descendant, watch, timer, queue,
   cancellation token, transient file/reference, UI generation, and receipt.
6. Project one nonmodal operation card plus a detailed state view with accessible
   progress and final announcements. Avoid global notifications unless the
   owning pane is unavailable and action is required.
7. Test every transition with a fake clock/scheduler, exact argv/environment,
   forbidden second mutation, provider/external-state oracles, timeouts,
   partial success, crash/restart, process trees, resource cleanup, native
   provider fixtures, package identity, and rollback/no-rollback behavior.

### Managed diagnostic-session experience

Port forwards, controlled probes, and workload debugging use the same PO6
preflight and operation monitor. They are not ordinary completion acceptance.

#### Kubernetes port forward

```text
Port forward Service/postgres
PRODUCTION · core-prod/payments

Listen       127.0.0.1:5432
Target       service/postgres:5432 · UID 3a...9c
Follow       Off
On close     Stop with this pane

[Cancel]  [Start managed port forward]
```

The active card shows local endpoint, resolved backend, environment, owner,
health, interruptions, credential expiry, and `Stop`. `Detach` and `Follow
resource` are separate reviews. A non-loopback listener is refused initially
for production rather than hidden behind a weak warning.

#### Controlled network probe

The review names source vantage point, destination, protocol, exact tool/image,
maximum traffic, deadline, permissions, data retained, and cleanup. Opening a
Network Path view never launches the probe. The result updates only the tested
layer and preserves every untested layer as unknown.

#### Debug workload

```text
Debug Pod/payments-api-739cb

Strategy     Debug copy
Image        registry.example/debug-tools@sha256:...
Profile      restricted
Exposure     production network · service account reference · no Secret values shown
Cleanup      delete copied Pod on reviewed exit; verify deletion

Alternatives
Ephemeral container — modifies current Pod and cannot be removed
Node debug — unavailable in the initial release

[Cancel]  [Start managed debug session]
```

The copy path does not say `original untouched therefore safe`: a copied Pod can
still consume resources or contact production dependencies. Arbitrary image
input, privileged profiles, node debugging, automatic target replacement, and
unverified cleanup remain unavailable until their own later gates pass.

Implementation reuses the activated D3/provider owner and adds typed session
descriptors, exact target/image/profile/listener/vantage contracts, one-use
grants, process/listener/temporary-resource ownership, context/auth revocation,
crash recovery, and content-minimized receipts. Each operation type needs
separate native/provider, privilege, admission, timeout, orphan, low-resource,
disable/uninstall, and cleanup evidence.

## PO7 — organization packs and guided workflows

### Organization packs: what the user sees

A pack review uses plain language:

```text
Payments operations pack · Example Corp · version 4

Adds: 12 service definitions, 8 diagnostic rules, 5 reviewed actions
Can do: contribute data and registered action definitions
Cannot do: run code, access credentials, call networks, or execute commands

[Cancel]  [Install disabled]
```

Guided workflows show one step, its owner, and why it is next. The next mutation
does not unlock until the current step is complete and the user returns to the
review surface.

### Organization packs: simple interaction

- Installation defaults to disabled and shows publisher, signature/provenance,
  version, compatibility, expiry, permissions, and changes from the installed
  version.
- Conflicting organization rules show both sources and refuse to choose
  silently.
- A workflow uses `Diagnose`, `Review action`, `Verify`, and `Recover` steps;
  native tool/source names remain visible.
- Disable, rollback, and uninstall are available from the same pack detail and
  state exactly what data will be removed.

### Organization packs: implementation requirements

1. Define strict signed/versioned declarative schemas for services, owners,
   dependencies, criticality, environments, evidence predicates, registered
   actions, risk, approvals, windows, success/stop/timeout/recovery, links,
   compatibility, and expiry.
2. Reuse the accepted D7 verification, provenance, capability, atomic install,
   generation, disable, rollback, and uninstall owners after their release gates.
3. Prove the schema cannot express scripts, callbacks, shell fragments, network
   requests, credential reads, capabilities, provider handles, or execution
   grants.
4. Add deterministic conflict/precedence and source display. Organization
   criticality can affect ranking only when the exact reviewed source is active.
5. Model a guided workflow as one visible step and typed outcome at a time. A
   state transition cannot execute the following mutation.
6. Test hostile bundles, duplicate keys, traversal/links/bombs, graph cycles,
   revoked publishers, downgrade, conflicting policy, unsupported actions,
   resource ceilings, native packaging, rollback, disable, uninstall, and zero
   capability residue.

## PO8 — adapter expansion and optional local tie-breaker

### Adapter expansion: what the user sees

New adapters appear only through the same passport, situation list, detail,
preflight, monitor, and settings. They do not add custom dashboards or competing
completion windows.

Adapter settings show `Enabled`, `Last refreshed`, `Scopes`, `Data retained`,
`Permissions`, `Health`, and `Disable/remove`. Optional local ranking shows:

```text
Local tie-breaker: Off
When enabled, it may reorder already-safe equal candidates. It cannot create or run actions.
```

### Adapter expansion: simple interaction

- An unavailable adapter removes or marks only its own evidence.
- Users can see which adapter produced each field and candidate.
- Enabling an adapter previews scopes and limits before activation.
- Enabling the local tie-breaker is explicit, removable, and never required.
- `Compare environments` first shows the reviewed service-equivalence mapping,
  selected environments, independent authorization/freshness, field/telemetry
  coverage, and fan-out limit. It is read-only and never creates a global
  restart, rollback, failover, or other multi-region action.

### Adapter expansion: implementation requirements

1. Deliver each adapter as an independent versioned slice with a clear evidence
   schema, capabilities, scopes, quotas, freshness, redaction, lifecycle,
   failure, native support, license, provenance, advisory, and uninstall review.
2. Keep provider SDKs/clients out of the core and pure model. Prefer official
   CLIs or current project infrastructure until structured API use has a measured
   benefit and a dependency ADR.
3. Add adapter conformance tests so every source maps unknown, stale, partial,
   denied, throttled, offline, and replaced state consistently.
4. Add read-only multi-region/cluster comparison only for explicitly mapped
   services. Bind every environment independently; normalize versions,
   configuration, dependencies, traffic, SLO summaries, time windows, and
   missing coverage without merging authority.
5. Evaluate a small local tie-breaker only after deterministic PO1-PO7 behavior
   is complete. Input is bounded redacted numeric/categorical features from
   already-valid candidates; output is a permutation, never text or an action.
6. Enforce CPU, memory, latency, model-size, provenance, update, disable, and
   deterministic-fallback limits. Any invalid/slow result is discarded.
7. Complete per-platform native UX/accessibility/package/resource/soak evidence
   for each support claim. An adapter can ship on fewer platforms than the core
   only when documentation and runtime availability say so clearly.

## End-to-end journeys

### Journey A — diagnose a failed rollout

1. The passport says `PRODUCTION · core-prod · payments · fresh 3s`.
2. The user types `kubectl rollout` and requests completion.
3. The situation group places `status` first because a rollout is active.
4. The user reviews details: revision 48 is new, but causality is not yet strong.
5. The user inserts `kubectl rollout status ...`; Enter remains shell-owned.
6. New evidence shows progress deadline exceeded and all new replicas unavailable.
7. `undo` becomes a review-required candidate with GitOps and recovery detail.
8. The user opens production preflight, sees exact target/UID, permission, impact,
   sync policy, stop signal, timeout, and recovery.
9. Before PO6, the only controlled outcome is reviewed insertion. After PO6,
   an explicitly approved managed operation can execute, observe, stabilize, and
   verify.
10. The final receipt says what was verified and what remains unknown.

### Journey B — restart would be wrong

1. Unhealthy Pods show `ImagePullBackOff` with an unavailable image reference.
2. `restart` is not the preferred candidate.
3. The list offers read-only image/registry and rollout-history diagnostics.
4. Details say `Restart recreates Pods with the same unavailable image`.
5. The user keeps working without a modal or warning storm.

### Journey C — context changes during review

1. A production preflight is open for cluster A and target UID X.
2. The provider context changes to cluster B or the target is recreated as UID Y.
3. The primary action disables immediately; no silent retarget occurs.
4. The review says `Context changed` and shows the exact public diff.
5. The user cancels or adopts the new context and starts a new preflight.

### Journey D — evidence is offline

1. The adapter disconnects while the user is typing.
2. Typing and CP1 continue without delay.
3. The passport says `Evidence stale · Kubernetes unavailable`.
4. Cached read-only candidates remain visibly stale if policy permits;
   production managed mutations are unavailable.
5. `Retry refresh` is explicit and cancellable.

### Journey E — verification fails

1. A managed action launches once after final revalidation.
2. Monitoring reaches its deadline without the declared success signal.
3. The card says `Uncertain — the verification deadline ended while external
   state may still be changing`.
4. Automexia stops automatic progression and offers read-only evidence plus a
   reviewed recovery action if one exists.
5. No second mutation starts until the user opens and approves a new preflight.

### Journey F — explain and compare one broken replica

1. Error Navigation reaches a timeout section and offers an explicit
   `Investigate this resource` handoff without sharing terminal history.
2. `Explain state` shows one unready Pod, four healthy peers, identical image
   and configuration references, and an observed node pressure condition.
3. `Compare with healthy` explains the same-owner/same-revision cohort and
   highlights node/readiness differences while keeping causality unproven.
4. `What changed?` finds no workload revision in the bounded window and says
   which CI/GitOps source is unavailable.
5. The next action is node and capacity diagnosis, not a blind rollout restart.

### Journey G — diagnose a Service timeout

1. The user selects `Diagnose connection` from a recognized timeout.
2. The read-only path confirms DNS, Service ports, and three ready EndpointSlice
   backends; NetworkPolicy remains unknown and TCP is untested.
3. The user reviews a Pod-vantage controlled probe with exact destination,
   image/tool, traffic ceiling, permission, timeout, and cleanup.
4. After PO6 activation the probe runs once and reports TCP timeout from that
   vantage only; TLS and application protocol remain untested.
5. The result offers policy/routing inspection and never claims the Service or
   application is the cause.

### Journey H — explain a Pending Pod

1. `Explain state` reads the Pod scheduling condition and bounded supporting
   Events, resource requests, taints, topology, PVC, and autoscaler summaries.
2. The view groups observed rejection reasons and marks scheduler-plugin details
   unavailable.
3. It offers `Inspect autoscaler`, `Inspect node capacity`, and `Inspect
   requests`; it does not recommend reducing requests automatically.

### Journey I — safely debug a distroless workload

1. Normal shell access has no debugging tools, so the user chooses `Debug
   workload`.
2. Preflight compares approved ephemeral-container and copied-Pod strategies,
   immutable debug image, profile, service-account/configuration exposure,
   admission, cleanup, and production risk.
3. The user selects a restricted copied Pod; the UI still warns that it can
   reach production dependencies.
4. PO6 starts one managed session, monitors it, and verifies copied-resource
   deletion on reviewed exit. Cleanup uncertainty remains visible.

### Journey J — hand over an incident

1. Incident Mode contains the objective, impact, current facts, hypotheses,
   contradictions, missing sources, attempted actions, outcomes, and next step.
2. Trusted timeline entries can jump to structured records; approximate terminal
   anchors are labelled.
3. `Export handoff` previews exact included and redacted fields.
4. Markdown/JSON export contains no raw logs, terminal history, metric series,
   provider payloads, credentials, or Secret values.
5. The next operator can verify evidence age and unresolved questions instead of
   treating the handoff narrative as current truth.

## Onboarding, settings, and removal

Onboarding is one optional page, not a wizard:

- explain that PO uses explicit bounded refresh and keeps native commands;
- list detected but disabled adapters;
- offer `Keep native completion only` and `Configure read-only context`;
- do not request credentials in Automexia; link to provider-owned login flows;
- keep managed mutation, Incident Mode persistence, organization packs, and
  local tie-breaking disabled until their phases and policies permit them.

Settings are grouped by purpose:

| Group | Settings |
|---|---|
| Context | enabled adapters, scopes, explicit refresh, freshness display |
| Suggestions | show situation group, maximum visible rows, reviewed insertion policy |
| Investigation | enabled explain/change/comparison/network/SLO sources, explicit scopes, cohort/window limits, live-log default off |
| Safety | environment classification, managed-action policy, confirmation policy, external policy source |
| Incident | session-only default, optional protected retention/export |
| Managed sessions | port-forward/debug/probe availability, approved images/profiles, loopback policy, detach/follow policy; safe ceilings are policy-owned |
| Performance | adapter limits and diagnostics; safe ceilings are not user-increasable beyond policy |
| Accessibility | announcements, reduced motion, high contrast; system preferences remain defaults |

`Disable Production Operations` cancels requests, closes/parks watches, removes
PO candidates and surfaces, clears memory caches, and returns to CP1. `Remove
local PO data` previews exact Automexia-owned journal/pack targets and never
removes provider configuration, credentials, kubeconfig, shell history, logs,
or organization systems.

## Accessibility and language

- Use listbox/option semantics for candidates, status semantics for refresh and
  operation changes, dialog semantics only for true managed confirmation, and
  clear relationships between row, detail, blockers, and actions.
- Tab moves between components; arrows move within composite lists. Focus and
  selection remain separate, and Escape returns without a state change.
- Every icon and color has text or shape redundancy. `PRODUCTION`, risk,
  freshness, selected state, blocked state, progress, and final result remain
  understandable in monochrome/high contrast.
- Keep focus visible and unobscured. Support keyboard, pointer, touchpad, IME,
  screen reader, zoom/scale, long localized names, Unicode, combining text, bidi
  isolation, and reduced motion.
- Use verbs and concrete nouns: `Refresh evidence`, `Review impact`, `Insert
  reviewed command`, `Execute reviewed action`, `Stop monitoring`, `Recover`,
  and `Cancel`. Avoid `Proceed`, `OK`, `AI insight`, `smart fix`, or `optimize`.
- Do not announce every event. Coalesce rapid changes and announce the useful
  state, source age, target, risk, blocker, or final outcome.

## Perceived performance

The interface must feel immediate even when providers are slow:

- CP1 appears on its existing path and is never delayed by PO.
- A requested situation surface first shows cached state or `Refreshing…`, not
  a blank frozen popup.
- A newer editor/context generation cancels the older request.
- Background refresh uses a discreet passport state. It never raises a global
  notification merely because a source is slow.
- Long-running managed work shows in-context progress, elapsed time, timeout,
  last meaningful state, and cancel/stop choices.
- Stale last-known-good state is preferable to flicker, but it is never painted
  as fresh or used for a managed production mutation.
- Animation is unnecessary. If a restrained fade is used, reduced motion makes
  it immediate and no animation delays input, focus, publication, or dismissal.

The numeric CPU, memory, query, cache, frame, candidate, journal, timeout, retry,
and shutdown ceilings remain in the product specification and the
[proposed machine contract](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md#resource-and-lifecycle-limits).
UX tests measure interaction latency through the real controller,
layout, renderer, and editor bridge rather than an isolated ranking helper.

## Proposed implementation map

Exact names require ADR/owner approval, but ownership must remain this clear:

| Layer | Proposed location | Responsibility | Forbidden authority |
|---|---|---|---|
| Pure operations model | New private `automexia-operations-model` crate | Versioned passports, observations, graphs, candidates, risk, preflight, operation state, receipts, limits, deterministic digest/ranking | Renderer, PTY, shell, filesystem, process, network, credentials, provider SDK, persistence, model runtime |
| Domain rules | `automexia-devops/src/operations/` | Kubernetes/cloud/GitOps symptom rules and typed candidate construction | Direct rendering, shell evaluation, credentials, unregistered actions |
| Provider adapters | Existing first-party provider extensions | Explicit bounded observation/action mappings, quotas, redaction, provider-native authority checks | Core mutation, ambient credentials, unbounded streams, per-key work |
| App composition | `apps/automexia-terminal/src/automexia/operations/` | Route generations, refresh, caches, adapter scheduling, lock, review, confirmation, broker handoff, monitoring, persistence lifecycle | Shell strings, renderer-owned state, second runner |
| UI model | `automexia-ui-model/src/operations.rs` | Responsive immutable projections, semantic/accessibility tree, focus/selection state | I/O, credentials, provider logic, process work |
| Screen/controller | `apps/automexia-terminal/src/screen/operations.rs` | Input routing, modal/surface priority, focus restore, CP5/detail/preflight/monitor transitions | Provider calls in input handlers, PTY mutation |
| Renderer | `apps/automexia-terminal/src/renderer/operations.rs` | Draw passport, details, preflight, monitor, incident surfaces from immutable snapshots | Ranking, policy, provider access, persistence |
| Existing CP5 | Current suggestions source/controller/renderer | Authenticated editor request, listbox, selection, replacement-only insertion, dismissal | Managed execution, provider refresh, preflight policy |
| Existing D3/provider broker | Current app-owned exact-action path | Exact argv/API launch, child/request lifecycle, cancellation, result | Candidate invention, UI ranking, implicit repeat |

Each module needs a single lifecycle owner, bounded queues, generation-aware
publication, cancellation, redacted errors, feature-disable behavior, and exact
shutdown joins. Public configuration and schemas require migration, rollback,
disable, uninstall, and compatibility tests before activation.

## Implementation order inside each phase

Every PO phase follows the same small-slice order:

1. Freeze the user-visible state table and exact non-goals.
2. Add pure types, limits, and failing boundary/negative tests.
3. Add deterministic controller/state transitions with fake providers/clocks.
4. Add renderer-neutral layout/accessibility projections and exact fixtures.
5. Add one thin app composition path behind a disabled feature gate.
6. Add real adapter/native integration for the smallest supported scope.
7. Measure input latency, provider cost, memory, handles, tasks, storage, and
   cleanup on named hardware.
8. Add failure, rollback, disable, uninstall, and migration evidence.
9. Run controlled pixels, keyboard/IME, screen readers, and provider fixtures on
   every claimed platform.
10. Update status only after the exact artifact and external gates are recorded.

## UX definition of done

A PO surface is not complete until all applicable statements are proven:

- a new user can identify environment, target, reason, risk, freshness, and the
  next safe interaction without documentation;
- the common read-only path takes one completion request and one selection;
- advanced evidence is one explicit action away and never required to dismiss
  or continue typing;
- there is one emphasized action, a visible safe exit, and no ambiguous `OK`;
- keyboard, pointer, IME, focus, screen reader, high contrast, scaling,
  localization, tiny/large layouts, and reduced motion behave consistently;
- no refresh steals focus, edits the buffer, sends PTY bytes, presses Enter, or
  opens an unexpected dialog;
- stale, partial, conflicting, denied, offline, deadline-reached, uncertain,
  cancelled, failed, and recovery states use truthful plain language;
- every comparison names cohort, exclusions, window and coverage; every active
  probe names its vantage point; approximate time and cross-source log order are
  labelled;
- state explanation preserves observed fact, assessment, recommendation, and
  action as separate user-visible concepts;
- manual shell insertion and managed execution are visibly and technically
  distinct;
- resource and latency ceilings pass through the real UI path;
- disabling/uninstalling removes PO surfaces and owned state while CP1 and the
  terminal continue normally; and
- native/provider/visual/accessibility evidence is attached to the exact
  supported artifact instead of inferred from mocks or source.

## Primary UX sources reviewed on 2026-08-25

- W3C's [combobox](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/) and
  [listbox](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/) patterns inform
  popup, selection, Escape, arrows, Home/End, and accessible naming behavior.
- W3C's [keyboard interface guidance](https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/)
  informs Tab-versus-arrow focus ownership.
- [WCAG 2.2](https://www.w3.org/TR/WCAG22/) constrains visible/unobscured focus,
  target size, predictable input, error identification, component semantics,
  and status messages.
- Microsoft's [progress controls guidance](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/progress-controls)
  supports nonblocking in-context progress with explanatory text.
- Microsoft's [dialog guidance](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/dialogs-and-flyouts/dialogs),
  Apple's [alert guidance](https://developer.apple.com/design/human-interface-guidelines/alerts),
  and GNOME's [dialog guidance](https://developer.gnome.org/hig/patterns/feedback/dialogs.html)
  support sparse, deliberate, cancel-safe modal use.
- VS Code's current [Command Palette](https://code.visualstudio.com/api/ux-guidelines/command-palette)
  and [notification](https://code.visualstudio.com/api/ux-guidelines/notifications)
  guidance supports clear categorized commands, limited notifications, and
  in-context cancellable progress.
- GNOME's [keyboard shortcut guidance](https://developer.gnome.org/hig/reference/keyboard)
  supports collision review and platform-consistent action discovery.

These sources guide interaction behavior. Automexia's native renderer and
existing ownership boundaries remain authoritative for implementation.
