---
name: automexia-terminal-assurance
description: Design, implement, or verify tests and evidence for Automexia terminal behavior, including PTYs, parsers, rendering, input, panes, resize, Unicode, concurrency, lifecycle, security, resources, performance, persistence, and cross-platform contracts. Use before production editing when these paths can change and again before completion. Do not use as proof of native platforms or hardware that did not actually run.
---

# Automexia Terminal Assurance

Define the smallest evidence set that can fail for the real defect, then expand it to the boundaries and interactions affected by the change. Keep terminal input/output responsive, resource-bounded, generation-safe, and owned by one authority.

This skill complements `$automexia-feature-planning`. Load the planning skill first for a new behavior, architecture change, dependency change, or refactor. Use `$automexia-native-ux-review` for visible native UI, compositor, keyboard, IME, focus, scale, or assistive-technology evidence.

## Authoritative inputs

Read the affected sections of:

- `../../../docs/TESTING.md`
- `../../../docs/FEATURE-TEST-REINFORCEMENT.md`
- `../../../tests/assurance/feature-test-reinforcement-v1.json`
- `../../../tests/assurance/feature-matrix.json`
- `../../../docs/ARCHITECTURE.md`
- `../../../SECURITY.md`
- the owning implementation, real callers, existing tests, benchmarks, fixtures, and platform adapters

The Markdown reinforcement plan and its JSON mirror are coupled authorities. Update every affected feature entry, risk flag, interaction, oracle, evidence owner, and exit criterion. Run the contract checker and its mutation tests when these records change.

## Test-first contract

1. Reproduce the real user or system path before changing production behavior. The regression must fail for the observed reason.
2. Add the smallest deterministic isolation test only after the real path is represented; it complements rather than replaces the real-path regression.
3. Characterize legacy behavior before moving ownership or consolidating implementations.
4. Add boundary and negative cases that distinguish the intended invariant from a narrow example.
5. Implement only enough to satisfy the next contract, then run the focused gate.
6. Keep the regression as permanent evidence and update the feature assurance records.

If implementation is not authorized, produce the scenario and evidence plan without editing production code.

## Choose independent evidence

Use the closest useful layer, then retain a real-path check:

- unit tests for pure state, parsing, policy, layout, units, and limits;
- integration and conformance tests for crate, shell, PTY, provider, protocol, capability, and persistence boundaries;
- property tests and fuzz targets for parsers, structured input, Unicode, fragmentation, malformed data, and limit transitions;
- deterministic model/concurrency tests for ordering, cancellation, saturation, generation replacement, restart, and shutdown;
- renderer-neutral snapshots for semantic layout and draw data;
- controlled raster evidence for exact owned pixels when a pixel contract exists;
- native tests for PTYs, shells, windows, GPU presentation, clipboard, IME, accessibility projection, packaging, credentials, and process trees;
- benchmarks with correctness assertions for latency, throughput, allocations, memory, startup, sustained operation, and cleanup;
- repeated lifecycle tests for handles, threads, child processes, workers, caches, temporary files, logs, and storage.

Never compute expected results with the production mechanism under test. A mock proves only the mocked boundary. A cross-platform table or cross-compile cannot replace a native run.

## Scenario inventory

Read [references/terminal-scenario-matrix.md](references/terminal-scenario-matrix.md) for PTY, parser, resize, input, rendering, concurrency, security, resource, and persistence scenarios. Select every row whose invariant or interaction can change; explain exclusions rather than copying the entire matrix into every task.

At minimum consider zero, one, boundary-minus-one, boundary, boundary-plus-one, maximum, over-limit, large, repeated, fragmented, malformed, cancelled, stale, and concurrent inputs. Cover every affected state transition, failure transition, retry, disable, rollback, migration, recovery, uninstall, shutdown, and restart.

## Required terminal invariants

- Treat terminal output, shell metadata, paths, imported files, provider output, completions, control sequences, and AI-generated content as untrusted.
- Keep filesystem, provider, network, authentication, database, and extension work off input, PTY, resize, renderer, and startup hot paths.
- Publish state before waking the renderer. Reject stale route, session, pane, geometry, and generation results.
- Bound bytes, decoded dimensions, recursion, file counts, queues, tasks, history, caches, logs, storage, time, retries, concurrency, threads, handles, and child processes.
- Make cancellation, cleanup, shutdown, and ownership observable. A worker-body notification or `is_finished()` hint is not proof that native thread-local destruction completed.
- Keep foreign joins and blocking cleanup off the input and event thread. Retain cleanup ownership after timeout.
- Launch processes using typed executables and exact argument arrays. Do not evaluate structured actions through command strings or implicit Enter.
- Redact failures while preserving the operation and repository-relative owner needed to act on them.

## Evidence ladder

Read [references/evidence-ladder.md](references/evidence-ladder.md) before claiming completion. Run the narrowest failing regression first, then the owning target, affected integration and policy gates, applicable feature combinations, and repository-wide required checks. Stop broad repetition when current evidence passes and no new failure or uncertainty justifies more work.

For every filtered command, verify the real executed test names and nonzero test count. For generated images, reports, packages, or other artifacts, require a fresh artifact and inspect its relevant content.

## Test intent comments

Use names and assertions to express ordinary behavior. Add concise comments only when they preserve non-obvious intent: a boundary value, historical failure, real user path, authority boundary, mocked host observation, independent oracle, cleanup requirement, atomicity, redaction, concurrency ordering, or native setup sequence.

Do not narrate syntax, repeat the test name, add mechanical Arrange/Act/Assert labels, or use comments to hide oversized fixtures.

## Completion classification

Report the feature as complete only when every applicable exit criterion has current evidence. Leave it partial or external when native OS, hardware, account, assistive technology, signing, controlled performance environment, long campaign, or human assessment has not run.

State exactly what ran, where it ran, what artifact was inspected, and what remains unavailable. Never promise that no defect can escape.
