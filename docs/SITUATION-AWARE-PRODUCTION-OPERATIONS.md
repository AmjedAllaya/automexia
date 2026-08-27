# Situation-aware Production Operations

Status: planned public direction. It is not available for production use.

## Purpose

Automexia should help DevOps and SRE teams understand the environment they are
about to change, investigate current evidence, and choose a safe action with
less manual switching between tools. The feature is intended to answer simple
incident questions quickly: where am I, what is unhealthy, what changed, what
depends on it, what action is appropriate, and how will I verify recovery?

For example, after a user types a partial Kubernetes rollout command, Automexia
may present unhealthy workloads relevant to the current cluster and explain why
they deserve attention. The user remains responsible for selecting and
approving an action.

## User experience

- Show account, environment, region, cluster, namespace, and production risk in
  a compact environment passport before recommendations.
- Prefer a small ranked list with plain explanations over a long undifferentiated
  completion list.
- Separate observed facts, deterministic conclusions, uncertainty, and missing
  evidence.
- Let users inspect the evidence behind a suggestion without leaving the
  terminal workflow.
- Preview impact, authority, target, command arguments, and verification before
  a mutating action.
- Require explicit approval for production changes and never submit a command
  merely because the user accepted a completion item.
- Observe the result, help verify recovery, and make rollback or handoff easy.

## Architectural boundary

This belongs in the independently enabled DevOps/SRE extension. The extension
may use application-brokered capabilities for provider tools, authentication,
and bounded evidence collection. It must reuse terminal-owned completion,
context display, diagnostic navigation, and reviewed-execution surfaces instead
of creating competing versions of them.

The terminal core remains provider-neutral. Provider SDKs, credentials,
network discovery, incident integrations, and organization policy stay outside
the input, PTY, rendering, and startup hot paths.

## Decision model

Recommendations should be based on inspectable rules, declared policy, current
bounded evidence, dependency relationships, blast radius, confidence, and
action prerequisites. Automexia must not claim to know the one correct action
when the evidence does not support that conclusion.

Small local models may later assist narrowly defined classification or
tie-breaking when explicitly enabled. They cannot replace policy, approvals,
provider truth, or deterministic safety gates. LLM access remains a separate
optional orchestration extension.

## Public delivery direction

The public order is environment awareness, safe read-only investigation,
diagnostic suggestions, impact and authority review, incident navigation, and
finally reviewed action-and-verification workflows. Organization packs and
additional providers follow only after the extension and capability boundaries
are proven.

Detailed ranking formulas, provider playbooks, schemas, internal limits, and
phase recipes are kept outside the public repository until implementation and
publication review.

## Security and privacy threat model

Provider output, logs, terminal text, imported policy, model output, paths, and
identifiers are untrusted. The feature must prevent command injection, secret
disclosure, stale or cross-environment publication, target substitution,
authority confusion, automatic execution, policy bypass, and persistent raw
evidence. Trust-boundary failures refuse the operation and preserve a manual
native-tool path.

## Resource and performance contract

Collection, comparison, suggestions, caches, queues, concurrency, retries,
network requests, retained evidence, storage, workers, handles, and child
processes remain bounded. Slow or unavailable guidance cannot delay typing,
terminal output, resize, rendering, startup, or shutdown. Exact internal limits
require implementation measurements before publication.

## Failure and refusal behavior

Stale context, incomplete evidence, denied authority, policy refusal, provider
failure, cancellation, and exceeded limits are visible outcomes, not reasons to
guess. Automexia keeps the prompt usable, explains what is missing, avoids
mutations, and offers a safe route back to native commands. Disable and uninstall
remove the optional behavior without damaging core terminal state.

See [Production Operations UX](SITUATION-AWARE-PRODUCTION-OPERATIONS-UX.md),
[contracts](SITUATION-AWARE-PRODUCTION-OPERATIONS-CONTRACTS.md),
[testing](SITUATION-AWARE-PRODUCTION-OPERATIONS-TESTING.md), and the
[Roadmap](ROADMAP.md).
