---
name: automexia-integration-delivery
description: Resolve Automexia merge, rebase, or cherry-pick conflicts and prepare validated changes for review, commits, an authorized push, or final handoff. Use for integration conflicts, coherent staging, DCO commits, documentation synchronization, remote verification, or completion review. This skill never treats implementation authority as permission to commit, push, rewrite history, publish, release, delete, or discard unrelated work.
---

# Automexia Integration Delivery

Integrate semantics rather than file versions, preserve every contributor's unrelated work, and produce a reviewable change whose tests, documentation, history, and remote state match the claim.

Load `$automexia-feature-planning` before changing behavior or architecture discovered during integration. Load `$automexia-terminal-assurance` and `$automexia-native-ux-review` for affected runtime and UI contracts. Load `$automexia-release-readiness` only for release-candidate or publication preparation.

## Authority boundaries

- A request to resolve conflicts authorizes conflict resolution and the validation required to prove it; it does not automatically authorize aborting the operation, resetting, dropping commits, rewriting shared history, or pushing unless the user also requested those actions.
- A commit request authorizes coherent commits, not a push.
- A push request authorizes a non-destructive push that preserves shared history after local validation. It does not authorize force push, release publication, tag movement, or deployment.
- Documentation or review work does not authorize production changes.
- Never stage, format, regenerate, move, or delete unrelated dirty files to make the tree appear clean.

## Starting-state record

Before integration or delivery, record:

- current branch, HEAD, upstream, operation in progress, and operation metadata;
- merge/rebase/cherry-pick parents or base when applicable;
- staged, unstaged, untracked, unmerged, renamed, deleted, and generated files;
- task-owned paths and unrelated paths that must remain untouched;
- applicable user authorization for resolve, continue, commit, push, publish, or release;
- current required checks and unavailable external evidence.

Re-read every target and its diff immediately before editing or staging it. A shared directory is not exclusive ownership.

## Conflict resolution

Read [references/conflict-resolution.md](references/conflict-resolution.md) for every unmerged path. Inspect the base, both sides, contributing commits, authoritative owner, callers, tests, generated source, and coupled configuration before choosing a result.

Do not resolve a file wholesale with ours or theirs unless evidence proves the complete file from that side is authoritative. Preserve deliberate removals and independent changes. Reconcile behavior, dependencies, schemas, permissions, pins, generated outputs, and documentation as a system.

After resolution, verify there are no unmerged entries or conflict markers, compare the result with the base and both parents, and run affected gates before continuing the Git operation.

## Review and validation

Use [references/delivery-checklist.md](references/delivery-checklist.md). At minimum:

1. inspect the complete task-owned diff and staged diff;
2. verify no unrelated changes were included or overwritten;
3. run whitespace, conflict-marker, formatting, documentation, privacy, secret, architecture, and targeted behavior checks applicable to the change;
4. run the owning tests and required repository profile after the final edit;
5. update implementation truth in tests, specifications, ADRs, roadmap/audit, user/contributor docs, and change fragment as applicable;
6. classify native, account, hardware, signing, human, and long-duration evidence honestly;
7. scan every task-owned changed or generated artifact for confidential and machine-local information before handoff.

Do not weaken a gate or omit a failure to obtain a clean delivery.

## Commit grouping

When commits are authorized:

- group changes by coherent review and rollback boundary;
- separate unrelated behavior, mechanical moves, dependency changes, and documentation when practical;
- stage exact paths or hunks after rereading them;
- use accurate imperative messages that describe the resulting behavior;
- include the repository-required DCO sign-off;
- never include secrets, private paths, usernames, hosts, provider output, or unrelated files in commits or messages;
- inspect each created commit and its parent before proceeding.

If another contributor changes a task-owned file during staging, unstage only the task-owned path if safe, reconcile the new content, rerun affected checks, and restage the reviewed result. Do not restore an older copy.

## Push and remote verification

When push is explicitly authorized:

- fetch or inspect the remote state without discarding local work;
- preserve shared history and integrate upstream changes semantically;
- refuse non-fast-forward repair by force unless the user separately and explicitly authorizes the exact history rewrite after risks are concrete;
- push only the intended branch and commits;
- verify the remote branch resolves to the expected commit;
- report protected-branch, authentication, policy, or hosted-check blockers precisely;
- attach a created pull request to the task when the environment supports task artifacts.

A successful transport message is not sufficient; verify the remote identity.

## Final handoff

Report:

- resulting behavior and authoritative owner;
- files and coherent commit groups changed;
- conflicts and semantic decisions made;
- exact validation that passed and the environment that ran it;
- failures investigated and unresolved evidence;
- external gates and risks;
- commit and remote identities when authorized;
- unrelated working-tree changes left untouched.

Do not claim the repository is clean when unrelated changes remain, or complete when required evidence remains partial or external.
