# ADR 0009: Launch-time shell provisioning

- Status: Accepted
- Date: 2026-08-13

## Context

Automexia's prompt metadata, context rows, session cloning, path styling, and
icon-aware listings depend on repository-owned shell integration. Requiring a
separate platform script after a build made a successful application launch
ambiguous: the binary could be current while PowerShell, CMD, WSL, Bash, or Zsh
support was stale or absent. It also forced users to remember a restart after
installing the integration.

Verification commands must remain safe to run in automation and must not edit a
contributor's profiles. Launch commands, however, already express the intent to
start a fully functional interactive terminal.

## Decision

1. `cargo dev` and `cargo automexia` run shell provisioning after build/smoke
   succeeds and immediately before the Automexia process is spawned.
2. On Windows the phase prepares PowerShell, CMD, and every detected user WSL
   distribution (excluding Docker Desktop's managed internal distributions).
   On macOS/Linux it prepares Bash, Zsh, and user-local terminfo.
3. The installers are repository-owned, non-networked, unprivileged, and
   idempotent. They use source fingerprints plus installed-file and profile
   marker checks; an unchanged launch does not rewrite a profile or start WSL.
4. Changed, missing, or locally damaged generated integration is atomically
   replaced from the checked-out source. Marked profile blocks are added at
   most once and unrelated profile content is preserved.
5. Provisioning failure aborts process creation with the installer error. The
   launcher never silently opens a partially integrated fallback session.
6. `cargo ready`, `cargo check`, `cargo ci`, and verification scopes do not run
   the installer and therefore remain non-mutating.
7. Platform scripts retain force and uninstall surfaces for focused diagnosis
   and explicit removal, but they are not normal build or launch steps.
8. Unit tests pin launch ordering and command arguments. Platform contract
   suites use temporary homes/configuration roots to prove first install,
   unchanged no-op, damaged-file repair, marker uniqueness, and source parity.

## Consequences

- One Cargo command now produces a shell-integrated terminal without a manual
  installer or restart.
- Normal launches add only a small local fingerprint/file verification cost.
- WSL is started only when its integration is stale, missing, or its detected
  distribution set changes.
- Editing or deleting an installed integration file is self-healing on the next
  launch.
- Contributors can run the complete non-launching gate without modifying their
  interactive shell configuration.
