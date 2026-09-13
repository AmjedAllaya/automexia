### Fixed

- Retain private provider-file cleanup ownership when deletion fails, allowing a
  later revoke or shutdown to retry instead of losing track of the file.
  Retire handles immediately across revoke, disable, session, expiry and shutdown
  paths so cleanup failures cannot preserve or restore access.
- Replace the CMD resize fixture's typeahead-discarding wait with buffered,
  non-echoing input and verify every queued probe without changing output rows.
- Reject empty, stale and inconsistent QA reports; preserve valid XML during
  diagnostic redaction. Run independent tests after
  failures and smoke-test all enabled benchmark scenarios. Add explicit build
  storage-reserve checks without automatic cache deletion.
- Bound polling-benchmark waits, verify exact readiness identities and join
  producers before the next iteration instead of measuring detached workers.
- Prune build/private/tool caches before benchmark inventory traversal, and fail
  on unreadable source directories instead of silently omitting evidence.
- Apply pre-descent pruning to publication-document checks while preserving
  their distinct root-only scope and rejecting unreadable public directories.
