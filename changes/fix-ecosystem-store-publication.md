# Preserve committed extension inventory after storage failures

The package store now validates bounded candidate state and checked revisions
before mutation, publishes memory only after an acknowledged write, and reports
recovery required after uncertain filesystem outcomes. Recovery rejects identity
mismatches and invalidates stale revisions when membership changes. Persisted
state remains schema v1; recovered inventory does not grant Settings trust.
