---
name: automexia-integration-delivery
description: Resolve Automexia merge/rebase/cherry-pick conflicts and perform Git delivery through coherent DCO commits, authorized pushes, PR preparation, and remote verification. Use only when the workflow reaches integration or Git delivery; never infer permission to rewrite history, publish, release, delete, or discard work.
---

# Automexia Integration Delivery

Integrate semantics rather than file versions, preserve unrelated work, and produce a reviewable Git result whose validation and remote state match the claim.

Do not load this skill merely to write an ordinary final implementation handoff. Load `$automexia-feature-planning`, `$automexia-terminal-assurance`, `$automexia-native-ux-review`, or `$automexia-release-readiness` only when integration exposes work inside their scopes.

## Targeted authorities

Search and read only the sections applicable to the requested action in `../../../CONTRIBUTING.md`, `../../../docs/CI-ASSURANCE.md`, `../../../docs/DOCUMENTATION.md`, `../../../docs/DEVELOPMENT-CACHE.md`, repository workflows/branch policy, generators, and change-fragment rules. Do not load all delivery authorities for a simple commit or conflict.

Local and hosted validation are separate evidence. Verify current policy instead of assuming hooks/jobs ran.

## Authority boundaries

- Conflict resolution does not authorize abort/reset/drop/history rewrite/push unless separately requested.
- A commit request does not authorize push.
- A push request authorizes only a non-destructive intended push; it does not authorize force push, release, tag movement, or deployment.
- Documentation/review work does not authorize production changes.
- Never stage, format, regenerate, move, or delete unrelated dirty files to make the tree appear clean.

## Starting state

Record branch, HEAD/upstream, operation metadata when applicable, dirty/unmerged inventory, task-owned paths, unrelated paths to preserve, and the user's authorized delivery step. Use `git diff --stat`/`--name-only` before path-specific or hunk diffs. Do not ingest a full unrelated working-tree diff.

Re-read task-owned content immediately before staging or conflict resolution.

## Conflict resolution

Use [references/conflict-resolution.md](references/conflict-resolution.md) only when actual conflicts exist. For each unmerged path, inspect the base, ours/theirs, contributing intent as needed, authoritative owner, important callers/tests, and coupled generated/configuration owners. Avoid wholesale ours/theirs replacement without evidence.

After resolution, verify no unmerged entries/conflict markers remain, compare only the affected result against relevant parents/base, and run the gates invalidated by the resolved change.

## Review and validation

Use [references/delivery-checklist.md](references/delivery-checklist.md) for commit/push/PR delivery and apply only applicable sections.

- inspect the complete **task-owned** diff/staged diff, not unrelated repository changes;
- run checks required by the changed owners and any repository policy explicitly tied to this delivery action;
- do not rerun expensive checks that remain valid after later non-affecting edits;
- synchronize only specifications/docs/ADRs/change fragments whose truth changed;
- scan task-owned artifacts for confidential or machine-local information before authorized commit/push.

Do not weaken a gate or omit a failure to obtain a clean delivery.

## Commit grouping

When commits are authorized, group coherent review/rollback boundaries, stage exact paths/hunks after rereading them, use accurate imperative messages with required DCO sign-off, and inspect each created commit. Never include secrets, private identifiers, or unrelated work.

If another contributor changes a task-owned path during staging, reconcile the current content and rerun only checks whose inputs changed.

## Push and remote verification

When push is explicitly authorized, inspect/fetch remote state without discarding local work, preserve shared history, push only intended commits/branch, and verify the remote branch resolves to the expected commit. Do not use force repair without separate explicit authorization for the exact rewrite.

## Handoff

Report the resulting behavior/owner, task-owned files/commits, conflict decisions when any, exact validation and environment, unresolved/external evidence, remote identity when authorized, and unrelated work left untouched when relevant. Keep the report concise; do not reproduce large diffs or logs.
