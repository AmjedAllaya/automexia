# Automation Studio testing

Status: planned public assurance summary.

Automation Studio requires evidence beyond editor unit tests. Before release,
the implemented path must cover:

- open, edit, undo, redo, save, save-as, conflict, recovery, and large-file
  behavior;
- malformed encodings, control characters, symlinks, permissions, read-only
  files, disk-full and interrupted writes;
- exact interpreter and tool arguments with no shell-string evaluation or
  implicit execution;
- language-service startup, crash, restart, cancellation, stale result rejection,
  saturation, and shutdown;
- bounded memory, CPU, file handles, child processes, caches, temporary files,
  history, and long-session growth;
- keyboard-only operation, focus movement, IME, localization, high contrast,
  reduced motion, narrow panes, high scale, and native screen readers;
- disable, uninstall, migration, external-editor fallback, and document recovery;
- domain-extension contributions that cannot escape their declared capabilities.

Visual changes need renderer-neutral state checks, deterministic raster evidence,
and native review on every claimed platform. Exact test matrices and thresholds
remain local until tied to an implemented dependency and reproducible fixtures.
