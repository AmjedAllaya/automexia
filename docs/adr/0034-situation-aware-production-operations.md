# ADR 0034: Situation-aware production operations boundary

- Status: Proposed
- Date: 2026-08-25
- Owners: terminal core, desktop application, extension platform, DevOps/SRE
  extension, security, product, release engineering

## Context

Automexia already has shell-native completion, a preview-disabled native editor
bridge, provider-neutral environment context, cached provider sources, typed
Quick Actions, and a nonactivated exact-argument execution path. Operators still
need to assemble live health, ownership, dependencies, permissions, GitOps,
production policy, and recovery information by hand before choosing a command.

The requested experience is broader than dynamic syntax completion. For example,
after `kubectl rollout`, Automexia should be able to show relevant valid actions
and prioritize affected workloads using current evidence. A restart must not be
suggested merely because Pods are unhealthy: bad images, configuration,
scheduling, capacity, probes, dependencies, or a bad rollout may require a
different action. Production state also requires current identity, authority,
policy, impact, approval, monitoring, and recovery.

Placing this logic in the core would add provider and policy authority to hot
terminal paths. Implementing it as a `kubectl` plugin cannot extend the existing
`kubectl rollout` command path. Letting an LLM invent and execute commands would
conflict with Automexia's lightweight, optional-model, typed-action, and
human-control boundaries.

## Proposed decision

If accepted, Automexia will build the `PO0-PO8` situation-aware production
operations track as an optional first-party DevOps/SRE capability layered on the
existing CP1/CP5, D2/D6, CP4/M13, and D3 boundaries.

1. The terminal core remains provider-neutral and model-free. It owns no
   Kubernetes, cloud, GitOps, observability, incident, or organization policy
   logic.
2. A private pure operation model will own versioned environment, knowledge and
   evidence quality, change/ownership, explanation, comparison, network-path,
   SLO-summary, graph, assessment, recommendation, intent, typed action, safety
   decision, preflight, one-use grant, verification, monitor, managed-session,
   and receipt contracts. It will have no renderer, PTY, shell, filesystem,
   process, network, credential, provider, persistence, or model authority.
3. Optional provider/domain adapters will normalize bounded public observations
   and typed actions. They will declare exact capabilities, scopes, quotas,
   freshness, redaction, lifecycle, and native evidence.
4. The desktop application remains the only composition and execution root. It
   binds route, environment, resource UID, evidence, policy, expiry, and one-use
   confirmation before handing exact arguments to the existing runner when that
   runner is independently activated.
5. CP1 remains the immediate fallback. CP5 owns editor-state authentication,
   candidate presentation, and replacement-only insertion. `PO` performs no
   provider work per keystroke and cannot press Enter.
6. Deterministic hard gates and explainable lexicographic ranking are the first
   implementation. Missing or conflicting evidence causes diagnosis or refusal.
   Observed fact, assessment, recommendation, typed action, safety decision, and
   ranking remain separate types.
7. An Automexia-managed production mutation or active diagnostic session
   requires a context lock, current authorization, impact preflight,
   policy/change-window/GitOps checks, explicit confirmation, exact one-use
   grant, bounded observation/stabilization, and a declared recovery/cleanup
   state.
8. Organization runbooks/policies are signed, versioned, declarative data. They
   cannot add arbitrary scripts, credentials, capabilities, or execution.
9. Evidence caches are route-scoped, bounded, memory-only by default, contain no
   raw logs/time series, and are removed on disable/uninstall. A separately owned
   PO5 live-log view is bounded and memory-only with visible gaps; it is never the
   evidence cache or journal. The default action journal is session-only and
   content-minimized.
10. No LLM is part of this track. A later optional small local ranker may reorder
    only candidates that already passed deterministic gates; it cannot create,
    approve, or execute actions. Cross-domain LLM planning remains isolated in
    the optional LLM Orchestration extension.
11. Internal records are bounded Rust types. Public interchange uses strict
    duplicate-key-rejecting JSON Schema 2020-12, SemVer, RFC 3339 time and
    canonical SHA-256 digests. One-shot provider reads wrap existing exact-argv
    adapters. Sustained Kubernetes list/watch may adopt a maintained client only
    in the optional first-party adapter after an independent dependency review;
    PO0 adds no dependency.
12. Policy is normalized from external authorities. Automexia hard safety and
    external denies cannot be overridden; unknown authorization, admission,
    policy, GitOps, JIT, change-window or credential state blocks a production
    mutation. Simulation, dry run, diff and IAM review never become execution
    guarantees.

One coherent user experience composes six capability families: provenance and
change, resource explanation, comparison, network diagnosis, incident evidence,
and managed diagnostic sessions. This does not authorize one mutable monolithic
engine. CP5 remains the sole completion/editor owner, DN remains the terminal
diagnostic navigator, provider adapters own I/O, the application composes
route-scoped state, and D3/provider brokers own any later process, listener or
external request.

PO2 begins with read-only change/ownership/drift, resource/scheduling
explanation, healthy cohort/revision comparison, network-path inspection and
optional SLO summaries. PO5 may add bounded live-log/time/handoff behavior. PO6
may add Kubernetes port forwards, controlled probes and safe debug sessions only
as separately reviewed managed operations. PO8 may add read-only cross-region/
cluster comparison with explicit equivalence mapping and independent authority.

The canonical detailed contract is
[Situation-Aware Production Operations](../SITUATION-AWARE-PRODUCTION-OPERATIONS.md),
the exact proposed record/rule/provider/policy/lifecycle/configuration contract
and 2026 research decisions are in
[Production Operations PO0 contracts](../SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md),
its exact surface, journey, wording, focus, responsive, implementation, and UX
definition-of-done contract is the
[Production Operations UX and implementation blueprint](../SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md),
and its future evidence contract is
[Situation-Aware Production Operations testing](../SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md).

The accepted decision must preserve three different promises. Native shell and
reviewed insertion replace authenticated editor text only; the shell owns a
later Enter, so Automexia cannot guarantee enforcement, monitoring, or outcome.
Only a separately activated PO6 managed operation receives final revalidation,
structured broker execution, observation, stabilization, verification, receipt,
cancellation, and recovery. Policy can withhold Automexia-provided mutation candidates or a
managed route, but cannot stop an equivalent command typed directly into an
unrestricted shell. No product copy, UI state, test claim, or receipt may imply
otherwise, and no managed action may silently fall back to shell insertion.

## Alternatives considered

### Make Automexia an LLM-first autonomous terminal

Rejected. It makes provider/model availability a product dependency, increases
privacy and resource cost, weakens reproducibility, and cannot replace current
authorization, policy, impact, and recovery checks.

### Put situation logic in the terminal core

Rejected. It would couple renderer/input/startup-sensitive code to provider
clients, credentials, policies, and rapidly changing domain behavior.

### Implement only a kubectl plugin or shell completion script

Rejected as the product owner. Kubernetes plugins cannot extend existing command
paths such as `kubectl rollout`, and shell completion alone cannot own cross-tool
context, policy, GitOps, impact, monitoring, or recovery. Native tool completions
remain useful syntax sources.

### Query providers on every keystroke

Rejected. It creates latency, rate-limit, battery, privacy, reliability, and
cross-context risks. Explicit refresh and bounded list/watch or polling feed a
cached immutable snapshot instead.

### Use one opaque “priority score”

Rejected. Safety gates, causal evidence, business criticality, dependency order,
urgency, and freshness have different meanings. Hard gates plus visible
lexicographic dimensions are easier to audit and test.

### Automatically execute the highest-ranked action

Rejected. Ranking does not grant authority and incomplete evidence can produce a
wrong recommendation. Insertion, review, execution, observation, stabilization,
verification, and recovery remain separate user-controlled stages.

### Create a separate extension that duplicates CP5 and provider state

Rejected. It would introduce competing editor, UI, route, cache, and execution
authorities. `PO` must compose existing owners through typed contracts.

### Build one monolithic production-operations engine

Rejected. A shared pure contract is useful, but a mutable service that owns
provider context, evidence, completion, terminal history, policy, incident state,
listeners and execution would create a confused deputy and duplicate existing
owners. Capability families compose through versioned snapshots and narrow
handoffs instead.

### Adopt a full observability or automation platform inside Automexia

Rejected. Automexia should integrate with current sources through bounded
adapters, not ingest and retain an organization's raw logs, metrics, credentials,
or control plane.

## Consequences

Positive:

- production value is added without making the core heavy or model-dependent;
- users keep native commands, visible evidence, and final control;
- Kubernetes, cloud, GitOps, observability, and organization policy can evolve as
  replaceable adapters;
- deterministic refusal and fallback keep ordinary terminal work reliable; and
- exact ownership, limits, and lifecycle rules are testable.

Costs and limitations:

- the feature cannot guarantee a single correct action;
- useful ranking depends on fresh, permitted, well-mapped evidence and reviewed
  organization metadata;
- real providers, clusters, GitOps systems, identities, native shells, and
  accessibility environments require expensive controlled evidence;
- execution cannot ship before D3 and provider capabilities are independently
  activated; and
- adapters and runbook schemas require long-term compatibility, security,
  provenance, and update ownership.

## Acceptance conditions

This ADR remains proposed until protected owners approve:

- exact crate/module ownership and dependency direction;
- the current proposed machine-contract digest, strict schema/compatibility
  policy, provisional numeric ceilings, semantic checker and mutation suite;
- capability, privacy, provider, GitOps, JIT, policy-pack, journal, and execution
  boundaries;
- change/ownership/drift, explainer, cohort, network-vantage, SLO, live-log/time,
  port-forward/probe/debug, and multi-environment comparison boundaries;
- deterministic ranking/refusal semantics and independent test oracles;
- disable, rollback, uninstall, and migration behavior; and
- the phase-specific native, provider, resource, accessibility, security, and
  release evidence ladder.

Acceptance does not activate a provider, watcher, completion source,
investigation view, live-log controller, managed diagnostic session, journal,
UI, model, or execution path.
