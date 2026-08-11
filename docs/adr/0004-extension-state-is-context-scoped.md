# ADR 0004: Extension-derived terminal context is scoped and bounded

Status: Accepted

## Decision

Extension-derived context that depends on a terminal's working directory is cached by context key rather than in one global snapshot. The v0.3 DevOps runtime uses a bounded per-working-directory cache.

## Why

A single application can contain multiple windows, tabs and splits. One global DevOps snapshot allows the most recently refreshed pane to overwrite every other pane's model. Context-scoped caching makes the data model correct for concurrent terminal surfaces while retaining a shared background worker.

## Consequences

- renderers request and read the snapshot matching their own CWD;
- the cache has a fixed entry limit;
- global generation changes may cause an unrelated renderer to perform a cheap cache lookup, but not IO;
- future session IDs can replace/path-augment the key without changing the renderer/extension ownership boundary.
