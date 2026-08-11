# Rio upstream strategy

Automexia is maintained as a controlled downstream terminal product. Upstream development is a source of fixes and ideas, not an automatic input to stable builds.

## Stable rule

Each Automexia release pins one exact audited Rio commit. `BOOTSTRAP-WINDOWS.ps1` branches from that SHA directly.

Do not build a release branch from `origin/main` merely because it contains the audited commit; doing so makes builds change when the fork moves.

## Updating the engine

A maintainer update should be explicit:

1. create a compatibility branch from the proposed new Rio SHA/tag;
2. apply Automexia integration;
3. run architecture, patch, warning, format, check, test and release gates;
4. run terminal protocol and UI smoke tests;
5. review performance/timing changes;
6. only then update the pinned SHA for a new Automexia release.

Do not merge upstream `main` into the stable Automexia release branch on a schedule. `AUDIT-UPSTREAM-WINDOWS.ps1` is intentionally read-only: it fetches and reports commits beyond the pin without merging, rebasing, resetting or checking out upstream code.

## Standalone future

Once the complete source tree is published as `automexia-terminal`, the preferred model is:

```text
origin        Automexia repository
rio-upstream  reference remote only
```

Useful upstream changes should be reviewed and ported/cherry-picked intentionally with attribution preserved.
