---
name: automexia-integration-delivery
description: Resolve Automexia merge/rebase/cherry-pick conflicts and deliver validated changes through review, coherent DCO commits, authorized pushes, PR preparation, and verified handoff. Use for integration conflicts or Git delivery; never infer permission to rewrite history, publish, release, delete, or discard work.
---

# Automexia Integration Delivery

Integrate semantics rather than file versions, preserve every contributor's unrelated work, and produce a reviewable change whose tests, documentation, history, and remote state match the claim.

Load `$automexia-feature-planning` before changing behavior or architecture discovered during integration. Load `$automexia-terminal-assurance` and `$automexia-native-ux-review` for affected runtime and UI contracts. Load `$automexia-release-readiness` only for release-candidate or publication preparation.

## Current authorities

Read the applicable current sections of `../../../CONTRIBUTING.md`, `../../../docs/CI-ASSURANCE.md`, `../../../docs/DOCUMENTATION.md`, and `../../../docs/DEVELOPMENT-CACHE.md`, plus the active Git operation metadata, repository workflows, branch policy, generators, and change-fragment rules. Use their current commands and policy instead of copying historical commands from a prior handoff.

Local validation and hosted validation are separate evidence. Verify whether current policy activates a local pre-push hook instead of assuming it runs; an inactive hook is not evidence, and an exact-commit hosted result remains separate from a local pass.

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

Use [references/delivery-checklist.md](references/delivery-checklist.md) and apply only the sections authorized by the user. At minimum:

1. inspect the complete task-owned and staged diffs and verify that unrelated work remains untouched;
2. run the applicable owning tests, repository profile, static, architecture, documentation, privacy, secret, and policy checks after the final edit;
3. synchronize specifications, tests, ADRs, roadmap/audits, contributor and user docs, and change fragments that own the resulting truth;
4. classify unavailable native or external evidence honestly and scan every task-owned artifact for confidential or machine-local information.

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
