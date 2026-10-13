---
name: automexia-code-review
description: Review an Automexia patch, commit range, or scoped subsystem for actionable correctness, maintainability, architecture, compatibility, and test gaps. Use for requested reviews and final self-review of non-trivial changes; native UX review and dedicated security audits have separate owners.
---

# Automexia Code Review

Find consequential defects by tracing behavior, not by counting patterns or proposing stylistic rewrites. A review request alone does not authorize fixing, staging, pushing, or publishing.

## Bind the review

1. Identify the requested scope and immutable base/head or current diff identity. Record existing dirty work and distinguish the task-owned patch from unrelated changes.
2. Read the diff summary before hunks. Inspect owning implementation, important callers, tests, feature gates, public contracts, and generated/upstream ownership. Expand only for a concrete dependency.
3. State the intended user-visible behavior and applicable invariants. Judge against those contracts and repository policy, not assumptions from the author's explanation.

Use [references/review-checks.md](references/review-checks.md) for the affected boundary. A full security audit uses the applicable available security skill; do not claim this general review replaces one. Report concrete security defects discovered in scope without publishing sensitive exploit material.

When reviewing agent instructions or skill routing, use [references/skill-workflow-review.md](references/skill-workflow-review.md). Judge the actions an agent would take in realistic cases, not merely whether a heading or keyword exists.

## Trace realistic failures

- Follow a normal request from entrypoint through state, side effects, result publication, and cleanup. Then trace relevant error, cancellation, retry, stale-result, and restart paths.
- Check ownership and ordering: who creates, mutates, persists, wakes, cancels, joins, and releases each resource? Verify route/session/pane/generation isolation where involved.
- Check compatibility: defaults, configuration precedence, units, errors, wire/storage versions, feature combinations, package contents, upgrades and rollback. Preserve intentional platform differences.
- Examine whether tests would fail if the claimed defect returned. Verify independent oracles, exercised entrypoints, nonzero test counts, negative controls, and evidence identity. Compilation, line coverage, or copied expected algorithms do not prove the contract.
- Evaluate complexity against actual consumers. Flag duplicate authority, unreachable fixes, unnecessary dependencies, or abstractions that obscure a real boundary; do not demand refactoring unrelated code.
- Search for callers before declaring code unused and inspect the generator before editing generated output. A deleted test or widened tolerance requires a contract-based explanation.

Use `$automexia-isolated-validation` if executing a reproduction is necessary. Otherwise state that the review used source inspection only. Debug a suspected issue with `$automexia-systematic-debugging` rather than presenting a guess as a confirmed finding.

## Report actionable findings

For each finding give:

- severity tied to impact, reachability, affected users, and recoverability;
- a precise file/symbol and smallest useful line range;
- a concrete trigger and the observed or logically traced failure;
- the violated contract and supporting code/test evidence;
- the needed correction or missing proof, without overprescribing an unverified design.

Use P0 for immediate critical impact, P1 for serious reachable failure requiring prompt correction, P2 for a material normal-priority defect, and P3 for a lower-impact actionable problem. Do not inflate speculative risks. Distinguish confirmed findings, questions, and optional suggestions; omit low-value nits unless requested.

Consolidate multiple symptoms of one cause. Recheck each finding against surrounding code and existing guards before reporting it. If no actionable findings remain, say so and identify material coverage limits; do not invent issues to fill a quota.

## Self-review after implementation

Make a separate pass over the final task-owned diff as if encountering it for the first time. Check whether the final code still meets the original acceptance criteria and whether fixes changed any previously passing evidence inputs.

When implementation is already authorized, correct verified findings within scope and rerun only invalidated checks in isolation. Otherwise deliver the findings without modifying production code. A self-review is not independent approval and must not be represented as a separate reviewer or CODEOWNERS sign-off.

Finish with findings first, then a concise account of reviewed scope, validation actually performed, and unresolved platform/external evidence. Do not bury a material failure under a positive summary.
