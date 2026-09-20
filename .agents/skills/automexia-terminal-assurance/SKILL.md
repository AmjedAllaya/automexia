---
name: automexia-terminal-assurance
description: Design and verify focused evidence for Automexia PTY, parser, rendering, input, resize, Unicode, concurrency, lifecycle, resource, security, performance, persistence, and cross-platform changes. Use when those contracts are affected; never infer native evidence from mocks or cross-compiles.
---

# Automexia Terminal Assurance

Define the smallest evidence set that can fail for the real defect, then expand only to affected boundaries and interactions. Keep terminal input/output responsive, resource-bounded, generation-safe, and owned by one authority.

Load `$automexia-feature-planning` first only when the change itself requires planning. Load `$automexia-native-ux-review` only when visible native UI, compositor, keyboard/IME/focus, scaling, or accessibility behavior is affected.

## Targeted authority lookup

Do **not** read these files in full by default:

- `../../../docs/TESTING.md`
- `../../../docs/FEATURE-TEST-REINFORCEMENT.md`
- `../../../tests/assurance/feature-test-reinforcement-v1.json`
- `../../../tests/assurance/feature-matrix.json`
- `../../../docs/ARCHITECTURE.md`
- `../../../SECURITY.md`

Instead:

1. identify the affected feature ID, source owner, test name, invariant, or heading;
2. use `rg -n`/structured extraction to locate only matching sections or JSON objects;
3. read directly referenced entries needed to understand the contract;
4. inspect owning implementation, important callers, existing relevant tests/benchmarks/fixtures/platform adapters;
5. expand to adjacent entries only when the change creates a concrete interaction.

Never load both complete assurance JSON files merely to find one feature. If the feature cannot be located reliably with bounded search/extraction, report that limitation and use the smallest safe fallback.

The reinforcement Markdown and JSON mirror remain coupled authorities. Update only affected entries/risk flags/interactions/oracles/evidence owners/exit criteria, then run their contract checker/mutation tests when those records change.

## Test-first contract

1. Reproduce the real user/system path or identify the existing real-path regression before production editing.
2. Add the smallest deterministic isolation test only when it provides distinct evidence.
3. Characterize legacy behavior before moving ownership or consolidating implementations.
4. Add only boundary/negative cases that distinguish the intended invariant from the observed failure.
5. Implement the smallest coherent change and run the focused gate.
6. Keep the regression as permanent evidence when it protects a real contract.

If implementation is not authorized, produce the scenario/evidence plan without editing production code.

## Evidence selection

Choose the closest useful layer and expand only when the claim requires it:

- unit/model tests for pure state, parsing, policy, layout, units, limits, ordering, cancellation, and generations;
- integration/conformance/native tests for PTY, shell, provider, protocol, process, persistence, platform, or capability boundaries;
- property/fuzz tests for hostile structured input, Unicode, fragmentation, malformed data, and limit transitions;
- renderer-neutral snapshots or controlled raster evidence for owned visual contracts;
- benchmarks/resource tests when latency, allocation, throughput, sustained operation, or cleanup is part of the change.

A mock proves only its boundary. A cross-compile cannot replace a native run. Never compute an expected result using the same production mechanism under test.

## Scenario inventory

Use [references/terminal-scenario-matrix.md](references/terminal-scenario-matrix.md) only for the affected domain. Read/select relevant rows; do not copy or execute the whole matrix for every task.

At minimum consider the boundary classes that can actually change: zero/one, limit edges, malformed/fragmented input, repeated use, cancellation/staleness, concurrency, failure/retry, cleanup/shutdown, and platform-specific behavior where applicable.

## Terminal invariants

- Treat terminal output, shell metadata, paths, imported/provider content, completions, control sequences, and AI-generated content as untrusted.
- Keep filesystem/provider/network/auth/database/extension work off input, PTY, resize, renderer, and startup hot paths.
- Reject stale route/session/pane/geometry/generation results and publish state before waking the renderer.
- Bound bytes, dimensions, recursion, files, queues, tasks, history, caches, logs, storage, time, retries, concurrency, threads, handles, and child processes.
- Make cancellation, cleanup, shutdown, and ownership observable; retain cleanup ownership after timeout.
- Keep foreign joins/blocking cleanup off input/event threads.
- Launch processes with typed executables and exact argument arrays; do not shell-evaluate structured actions.
- Keep diagnostics bounded, redacted, and content-minimal.

## Validation ladder and output discipline

Use [references/evidence-ladder.md](references/evidence-ladder.md) when more than a focused/owner-level check is needed.

1. During editing, run the narrow failing/characterization test.
2. After implementation, run the owning target and applicable static checks.
3. Before delivery, run only the integration/policy/platform/repository gates whose inputs were invalidated by the change.

Do not rerun an expensive passing check after a later edit that cannot affect it. Capture verbose output to a file when practical and return only pass/fail counts plus relevant failure excerpts; use full output only when it is required evidence. Verify filtered test names and nonzero counts.

## Completion

Call the affected contract complete only when its applicable exit criteria have current evidence. Leave native OS, hardware, account, assistive-technology, signing, controlled-performance, long-campaign, or human evidence explicitly partial/external when unavailable. State exactly what ran and what remains unavailable.
