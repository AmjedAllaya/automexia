# Stabilization direction

Status: active release work; this public page summarizes quality goals rather
than the internal execution ledger.

Automexia's first release is gated on demonstrated behavior, not a feature
count. The stabilization program covers:

- terminal protocol correctness and bounded hostile input;
- prompt, resize, input, PTY, pane, tab, and shutdown behavior;
- configuration reload, migration, rollback, and last-known-good recovery;
- memory, CPU, GPU, storage, handles, threads, processes, queues, caches, and
  long-session cleanup;
- deterministic rendering, themes, scaling, clipping, and native visual review;
- keyboard, IME, focus, high contrast, reduced motion, and screen readers;
- Windows, Linux/BSD, and macOS native evidence for every platform claimed;
- packaging, provenance, signing, update, release, and rollback procedures;
- documentation that clearly separates available, release-gated, and planned
  behavior.

Command productivity, SSH, provider integrations, and future extensions cannot
override these gates. A narrow unit test or cross-compile does not prove native
release readiness.

Detailed task ordering, internal thresholds, host-specific evidence, and
implementation-agent instructions remain outside the public repository. See
[Testing](TESTING.md), [Release Trust](RELEASE-TRUST.md), and
[Readiness Audit](READINESS-AUDIT.md) for public evidence and constraints.

## S0 bounded control strings

Hostile and oversized terminal control strings must remain bounded and covered
by parser, property, fuzz, and regression evidence before release.

## Verification infrastructure plan

Repository-owned checks should produce deterministic, reviewable evidence and
fail closed when required scenarios, platforms, artifacts, or owners are
missing. Native and human evidence stays explicit rather than being inferred
from a cross-compile or simulated result.

## Performance proof plan

Performance claims use same-host baselines and measure the real terminal,
renderer, session, provider, and extension owners. Evidence includes latency
distributions, throughput, allocations, memory, GPU memory where observable,
handles, threads, processes, queue and cache growth, storage, cancellation, and
final cleanup.

## S1 executable benchmark pipeline

Repository benchmarks must be reproducible, identify their environment and
artifact, retain the first failure, and compare against noise-aware thresholds.
Helper-only measurements cannot substitute for the interactive owning path.

## S1 accessibility baseline

Release evidence covers semantics, focus order and restoration, keyboard-only
operation, high contrast, reduced motion, scaling, and native screen readers.
Automated trees and events are necessary but do not replace controlled native
assistive-technology workflows.

## S2 enforcement

Release enforcement fails closed when required performance, resource, visual,
accessibility, security, native-platform, packaging, or review evidence is
missing, stale, tied to another revision, or outside its declared environment.

## Early DevOps and SSH delivery track

Remote-operation work follows terminal stabilization and preserves the same
security, lifecycle, accessibility, resource, and native-platform gates. See
the [SSH and multi-cloud delivery summary](SSH-CONNECTIVITY-MULTI-ENVIRONMENT-MULTI-CLOUD-PLAN.md).

## Command productivity delivery track

Completion, Quick Actions, aliases, and suggestions remain separate from
command execution. See [Command Productivity](COMMAND-PRODUCTIVITY.md) for the
current implemented contract and release limitations, and
[Quick Actions and aliases](DEVOPS-ALIASES.md) for the detailed shipped
behavior.
