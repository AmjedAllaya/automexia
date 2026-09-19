# Delivery checklist

Apply only the sections authorized by the user.

## Review-ready change

- Outcome and acceptance criteria still match the request.
- Task-owned diff is minimal, coherent, and architecture-aligned.
- Unrelated dirty work is present only as untouched context.
- Production behavior, tests, docs, ADRs, roadmap/audits, and change fragment agree.
- No unfinished placeholder, debug bypass, temporary compatibility owner, or obsolete copy remains without explicit ownership and removal criteria.
- Errors are typed, actionable, bounded, and redacted; recoverable input does not panic.
- Unsafe code is minimal, encapsulated, documented with invariants, and has appropriate review/evidence.
- No real username, host, profile path, environment value, credentials, private history, or identifying provider output entered an artifact.

## Validation

- Diff and whitespace checks pass.
- Conflict-marker and unmerged-entry checks pass.
- Skill and documentation validation pass when those files changed.
- Formatter and static checks pass for changed languages.
- New regression and legitimate control pass with a nonzero test count.
- Owning target and affected integration gates pass.
- Architecture, feature reinforcement, dependency, security, privacy, workflow, packaging, and release-policy gates pass when applicable.
- Required fresh artifacts were generated and inspected.
- Required repository profile passed after the final edit.
- Native and external evidence names only environments actually exercised.

## Commit

- User authorized commits.
- Staged paths and hunks were reread immediately before commit.
- Commit boundary is independently reviewable and revertible.
- Message describes final behavior and contains no private information.
- DCO sign-off is present.
- Created commit has the expected parent and tree.

## Push

- User explicitly authorized push.
- Upstream and remote target are unambiguous.
- Remote changes were fetched or inspected and integrated without discarding work.
- Push is fast-forward and targets only the intended branch.
- Remote branch resolves to the expected commit after push.
- Hosted checks, protection rules, or required reviews are reported separately from local validation.

## Handoff

State:

- what changed and why;
- architecture and reuse decisions;
- tests and artifacts inspected;
- native environments exercised;
- external or unavailable evidence;
- commit and remote identities when applicable;
- remaining risks or follow-up with exact owners;
- unrelated files left untouched.
