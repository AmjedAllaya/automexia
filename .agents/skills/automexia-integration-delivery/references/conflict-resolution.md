# Semantic conflict resolution

Use this procedure for merge, rebase, cherry-pick, revert, stash-apply, or patch conflicts.

## Capture the operation

Identify:

- operation type and state directory;
- current branch and HEAD;
- merge heads, rebased commit, cherry-picked commit, or applied patch;
- merge base and both relevant parents;
- index stages for each unmerged path;
- rename, delete, submodule, file-mode, and case-only changes;
- working-tree changes created before the operation;
- user-authorized next action: resolve, continue, abort, commit, or push.

Do not abort, reset, checkout another branch, clean, or rewrite until the exact work at risk is known and the user has authorized any destructive consequence.

## Resolve each path

For every conflict:

1. Inspect the base, ours, and theirs versions plus the contributing commits.
2. Identify the authoritative generator, schema, crate/module, test, documentation, or platform owner.
3. Inspect important callers and coupled files.
4. State the invariant each side was trying to preserve.
5. Choose the combined behavior that preserves compatible intent and current architecture.
6. Add or update tests when the merge exposes a behavioral interaction not independently covered.
7. Regenerate derived files from the reconciled source of truth.
8. Re-read the final path and its diff before marking it resolved.

## Special conflict classes

### Modify/delete

Determine whether the deletion was deliberate removal, replacement, generated cleanup, rename, or an obsolete path. Do not resurrect removed behavior merely because the other side edited it. Do not accept deletion if a still-live contract requires the owner.

### Rename or directory move

Trace both the old and new path, imports, manifests, tests, documentation, build scripts, packaging, and architecture checks. Apply content changes to the current authoritative path and remove stale references.

### Cargo manifests and lockfile

Reconcile dependency requirements, feature flags, target conditions, workspace inheritance, patches, profiles, and MSRV first. Regenerate the lockfile with the repository toolchain; do not hand-splice lockfile entries. Run dependency, license, vet/deny, feature, and relevant target checks.

### Workflows and action pins

Preserve least permissions, immutable reviewed pins, timeouts, concurrency, artifact identity, platform matrix, failure propagation, and repository policy. Validate YAML and run workflow policy/mutation checks.

### Generated files

Resolve the generator, template, source schema, or fixture first, then regenerate. Verify provenance and deterministic output. Never establish a hand-edited generated result as a second authority.

### Schemas, migrations, protocols, and persisted data

Preserve compatibility, versioning, validation, limits, migration order, rollback, partial failure, and old-reader/new-reader behavior. A conflict resolution that compiles can still corrupt or strand data.

### Tests and fixtures

Retain independent oracles and coverage from both sides. Reconcile expected behavior only after the production contract is established. Do not delete a failing regression because the merge is inconvenient.

### Documentation and roadmaps

Describe the integrated current state. Preserve deliberate limitations, external gates, and replaced decisions. Do not combine mutually exclusive future promises.

## Verification before continue

- No unmerged index entries remain.
- No conflict markers remain in task-owned text or generated output.
- The final diff contains the intended contribution from each side and no accidental wholesale replacement.
- Generated files match their owners.
- Manifests, schemas, docs, tests, and code agree.
- Focused tests cover newly interacting behavior.
- Applicable formatting, architecture, dependency, documentation, security, and policy gates pass.
- The operation will create the expected parent/history relationship.

Continue the Git operation only after these checks. If continuation creates further conflicts, repeat this procedure with the new commit rather than copying the previous resolution blindly.
