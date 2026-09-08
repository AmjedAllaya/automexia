# ADR 0052: Separate workload lifecycle from readiness colours

Status: Accepted

## Context

The optional DevOps row classifier treated every `Completed` or `Succeeded`
word and Docker `Exited (0)` as green success. Readiness was checked only for
`Running`. Generic word scanning could also let a resource name override its
status or let a zero-error count hide a nonzero failure count.

## Decision

Keep classification in the existing `automexia-devops` extension. Core
`grid_emit` only maps generic severity to configured terminal colours; no new
extension, enum, capability, dependency, worker or persisted setting is added.

Recognize bounded default Kubernetes/oc pod rows (`[namespace] name READY
STATUS`) before prose. Cyan means completed/informational, green means known
full readiness or health, amber means incomplete/unknown readiness or a
transition, and red means an explicit failure. Container clean exit is cyan;
nonzero exit is red. Paused and starting health are amber, even if an older
healthy label also appears. Absence of a health check is not proof of health.
Condition booleans respect polarity: `Ready=False` differs from
`DiskPressure=False`. Unknown table statuses do not fall through to a name-based
guess. Generic successful command summaries remain green.
Kind-prefixed `pod/name` rows retain the same readiness contract; arbitrary
slash-containing paths do not become resource identifiers. Numeric counts alone
are not log timestamps, so `0 error` cannot declare an error-level log.

Plain-row heuristics are presentation hints, not a replacement for tool status,
structured APIs or live health probes. Rows over 32 KiB are unclassified. The
classifier allocates no heap memory and cannot read files, execute commands,
connect to a provider, mutate cells or change command exit status. Source text
remains available to copy, search and assistive technologies. Explicit ANSI
colours and disabled-extension behavior retain their existing owners.

## Evidence and alternatives

This interpretation follows the distinctions in [Kubernetes Pod lifecycle](https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/),
[node conditions](https://kubernetes.io/docs/reference/node/node-status/),
[Docker container status](https://docs.docker.com/reference/cli/docker/container/ls/)
and [Docker health checks](https://docs.docker.com/reference/dockerfile/#healthcheck).
Kind-prefixed output follows [kubectl get](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_get/).
Using red for every completed job would incorrectly imply failure; keeping it
green would conflate successful termination with service readiness. Adding a
provider API or parser dependency to the renderer would expand authority and
latency without improving this bounded plain-output presentation contract.

Tests include lifecycle matrices, conflicting names, invalid ratios, summaries,
declared log levels, byte limits, properties, actual VT snapshots and grid glyph
colours, controlled CPU pixels, configured colours, ANSI preservation and disable.
The existing service benchmark owns before/after classifier measurements.
Controlled native desktop and screen-reader coverage remains external; CPU
pixel evidence does not certify every native GPU or shell/tool combination.

Rollback is a normal code revert. No configuration migration is required.
