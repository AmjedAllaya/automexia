# Extensions

Automexia's public extension documentation is a maintenance and security
boundary, not a product catalog or marketplace roadmap.

## Current public status

Automexia v0.4 does not provide public third-party download, installation,
enablement, or marketplace distribution.

The source may contain optional internal components. Their names and scaffolding
do not announce a public feature. Public behavior is limited to
[the feature catalog](FEATURES.md).

## Signed settings metadata source contract

The local verifier can decode optional `settings.v1.json` declarations from an
already verified package. The existing content signature covers that member;
manifest v1, signature framing, receipts, required members and release gates are
unchanged. This source API does not install settings, execute components or
promise that a declared option has an active consumer.

Schema 1 binds the verified publisher, extension and version. It accepts plain
feature labels and descriptions, Boolean enable defaults, and Boolean, Choice
or Integer options. Metadata is bounded to 64 KiB, 16 features, 8 options per
feature and 64 total controls including enable rows. Labels, descriptions,
local IDs and choices have separate limits. Defaults must match the declared
type, choices and integer range/step; unknown fields, duplicate identities and
unsafe text are rejected as a whole. The pure decoder reuses strict JSON parsing.

A package without declarations retains its previous behavior. Invalid optional
metadata returns a bounded typed Settings error without changing acceptance of
an otherwise valid legacy package. Signature, publisher or revocation failures
still reject the package. Call this projection on the verification worker, not
on terminal input or rendering paths. The literal example is maintained in
[`settings-metadata-v1.json`](../tests/fixtures/ecosystem/settings-metadata-v1.json).

## Ownership rules

1. Core owns terminal input, PTY/process lifecycle, VT state, rendering,
   windows, tabs, panes, configuration, clipboard, and stable capability
   contracts.
2. Optional code receives only declared bounded data and capabilities.
3. The application owns privileged filesystem, network, credential, provider,
   and process operations.
4. Optional work stays off terminal input, PTY, resize, renderer, and startup hot
   paths.
5. Results are validated, route/session/generation bound, cancellable, and stale
   results are rejected.
6. Disable and uninstall cancel and join work, remove only exact owned state,
   preserve user files, and leave the terminal usable.
7. Credentials remain with platform or external-tool owners.

## Capability model

A capability binds exact package/component identity, operation, scope, route or
resource, generation, expiry, and revocation state. Unknown, stale, widened, or
mismatched requests fail closed. No optional component receives ambient
terminal content, history, clipboard, environment, filesystem, network,
credential, process, or PTY authority.

## Contributor checklist

Before changing public extension infrastructure:

- identify the current contract, code, test, lifecycle, and packaging owners;
- keep one source of truth and preserve core dependency direction;
- bound bytes, dimensions, counts, depth, queues, concurrency, time, retries,
  caches, storage, and logs;
- use typed executables and exact argument arrays;
- treat package data, paths, metadata, and output as untrusted;
- test malformed, oversized, stale, cancelled, saturated, crash, restart,
  disable, uninstall, recovery, and shutdown behavior;
- verify no optional failure breaks the basic terminal; and
- update public docs only for explicitly published baseline behavior.

See [Architecture](ARCHITECTURE.md),
[Build, wrap, and adopt](BUILD-WRAP-ADOPT-ARCHITECTURE.md),
[Feature ownership](FEATURE-OWNERSHIP-AUDIT.md), and [Testing](TESTING.md).

Unreleased extension ideas, detailed architecture, integrations, services, and
commercial packaging remain private.
