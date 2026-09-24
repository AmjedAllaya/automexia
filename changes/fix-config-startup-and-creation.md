# Fix configuration startup and initial creation

- Validate complete configured environment batches before startup mutation or new session launch, preserving Unicode, empty values and embedded equals signs.
- Reject malformed global and platform environment overrides before publishing a replacement configuration.
- Stage complete starter configuration files and publish without overwriting existing files, returning explicit creation, existing-file and failure outcomes.
- Replace shared configuration and theme test paths with individually owned temporary directories and automatic cleanup.

Welcome configuration creation runs through one application-owned bounded worker.
Repeated Enter and another window's request cannot queue duplicate publication;
late results are rejected after route replacement, closure, or cancellation.
The initiating Enter remains consumed through release even if creation completes
immediately. The existing Welcome action displays creation progress, and final
shutdown gives the worker a finite completion budget. Already-entered filesystem
publication can finish after cancellation without changing an obsolete route.
Missing default configuration roots produce a content-free creation error.
