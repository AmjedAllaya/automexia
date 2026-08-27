# Situation-aware Production Operations testing

Status: planned public assurance summary.

Production guidance cannot be considered complete because a suggestion looks
plausible in a demo. Release evidence must cover:

- deterministic policy, ranking, freshness, and uncertainty behavior;
- incorrect, missing, hostile, oversized, and cross-environment provider data;
- exact target, executable, arguments, environment, approval, and forbidden
  side effects;
- cancellation, stale-generation rejection, saturation, provider failure,
  restart, shutdown, disable, uninstall, and recovery;
- bounded CPU, memory, network, cache, storage, handle, and process use during
  long sessions and incident bursts;
- keyboard, focus, responsive layout, high contrast, reduced motion, and native
  screen-reader workflows;
- native provider-tool behavior with public fixtures or isolated test accounts;
- complete end-to-end paths from context display through evidence, review,
  execution, observation, verification, and rollback.

No release claim may treat mocked provider results, cross-compilation, a short
fuzz run, or a narrow unit test as proof of production safety. Exact private
test matrices and internal thresholds are published only when they are tied to
implemented behavior and reproducible public fixtures.

## Scenario inventory

The public scenario classes cover boundaries, hostile input, stale and missing
evidence, provider failure, policy refusal, approval separation, cancellation,
recovery, accessibility, resources, native platforms, disable, and uninstall.
The exact unpublished fixture and provider matrix remains local.

## Phase exit criteria

The feature remains planned until implemented source, independent oracles,
native provider workflows, security review, accessibility, resource and
performance evidence, packaging, rollback, and documentation all agree. Missing
external evidence remains explicitly incomplete.
