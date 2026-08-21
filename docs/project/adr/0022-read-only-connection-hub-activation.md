# ADR 0022: Read-only Connection Hub activation

- Status: Accepted for v0.5 M1/D5.1
- Date: 2026-08-21
- Extends: [ADR 0012](0012-first-party-ssh-and-session-launch-boundary.md), [ADR 0013](0013-renderer-independent-accessibility-model.md), and [ADR 0020](0020-hybrid-build-wrap-adopt-boundary.md)

## Context

Automexia already owns a bounded, non-executing OpenSSH inventory, public
metadata compare-and-swap store, private Connection Library, catalog/query
projection, and renderer-neutral Connection Hub model. The application does not
initialize those stores, own the inventory worker through shutdown, let users
select exact source files, or render the Hub. The current worker also keeps an
immutable metadata snapshot and discards its join handle, so it cannot support
reviewed favorite/tag edits or prove deterministic teardown.

An exact file-selection flow must work on Windows, macOS, Linux, and supported
BSD desktops without building a second file browser or introducing raw Win32,
AppKit, GTK, or desktop-portal unsafe code into Automexia. Selection alone must
not become ambient filesystem authority: files still need D4 canonicalization,
link/reparse rejection, exact grants, immutable ceilings, explicit confirmation,
and no persisted raw path.

## Decision

1. Router owns one cloneable Connection Hub service for the application
   lifetime. It opens MetadataStore below extensions/devops-ssh and
   ConnectionLibraryStore below connections, then gives each Screen a
   route/window-local controller handle. Store initialization failure degrades
   to a fixed redacted unavailable/recovery state and never prevents the core
   terminal from starting.
2. The service owns one named worker and an explicit shutdown flag, scan
   cancellation handle, condition variable, and join handle. Dropping the last
   service handle cancels pending/current work, clears memory-only grants, wakes
   the worker, and joins it. Obsolete generations cannot publish.
3. `rfd` 0.17.2 is adopted with its reviewed default native/portal backend. The
   picker is opened only after an explicit Hub action, is parented to the current
   native window, supports cancellation, and returns exact selected files. It is
   not a discovery scanner and does not persist or remember paths.
4. Canonicalization, link/reparse validation, and InventoryGrant construction
   run on the Hub worker after selection. The controller presents bounded,
   bidi/control-safe canonical path labels for one explicit review. Only the
   reviewed generation can be confirmed for scanning, and a new selection
   revokes the previous grant.
5. Successful scans publish immutable records/catalog/state before invoking the
   exact originating route wake. Failures use stable diagnostic codes and retain
   a last-known-good catalog. No path, host, identity, tag, search, or provider
   value is logged.
6. Favorite/tag changes use only the existing D4 metadata schema and
   MetadataStore::compare_and_swap. The UI shows the public before/after diff,
   requires the reviewed revision, and reloads/recomposes on conflict. Recent
   timestamps cannot be written in M1.
7. The modal consumes the existing renderer-neutral Hub projection and is the
   sole topmost product surface while active. Background terminal input is
   inert, focus is trapped/restored, rows are virtualized, and keyboard,
   pointer, IME, accessibility names/actions, scale, high-contrast, and reduced
   motion/transparency behavior remain semantic rather than pixel-owned.
8. Profiles, recipes, and preferences are snapshot-only. Connect, login,
   provider refresh, automatic actions, process/network work, and PTY/session
   creation return a fixed visibly disabled result. ADR 0012 remains the
   protected prerequisite for later authority.

The dependency is MIT-licensed, uses raw-window-handle 0.6 and the same current
platform binding families already present in the workspace, supports Windows,
macOS, Linux, and BSD, and delegates Linux selection to the XDG desktop portal
with documented desktop-backend/Zenity fallback requirements. The reviewed
primary sources are the [`rfd` 0.17.2 crate documentation](https://docs.rs/rfd/0.17.2/rfd/),
[`FileDialog` API](https://docs.rs/rfd/0.17.2/rfd/struct.FileDialog.html), and
[upstream changelog](https://docs.rs/crate/rfd/0.17.2/source/CHANGELOG.md).
The changelog records the 0.17.2 MSRV reduction to Rust 1.88 and the preceding
0.17.1 aarch64 fix. Cargo policy, advisories, duplicate versions, lockfile,
build, and release-binary impact are verified in the M1 evidence ladder. An
unavailable platform picker leaves the Hub open with setup guidance and does
not fall back to scanning a candidate path.

## Alternatives rejected

- **Silently scan standard OpenSSH locations.** Candidate locations are
  guidance, not grants; ambient scanning violates D4's consent boundary.
- **Persist selected paths for convenience.** Paths disclose private machine
  structure and become stale authority without a source-change, revocation,
  privacy, and migration contract.
- **Build an in-app filesystem browser.** This duplicates mature native
  accessibility, sandbox/portal access, localization, and platform behavior and
  expands Automexia's filesystem authority.
- **Write direct Win32/AppKit/GTK/portal adapters.** This adds substantial
  platform-specific unsafe/FFI ownership, build dependencies, and testing burden
  for a mature commodity primitive.
- **Run a synchronous scan or canonicalization on the input/render thread.** It
  can delay terminal interaction and makes cancellation/stale-result handling
  unreliable.
- **Reuse the Connection Library for favorites/tags or grant paths.** D4
  metadata already owns those public inventory fields; a second owner would
  create conflicts and migration ambiguity.
- **Enable connect/login while the dialog is being added.** D5.2 and protected
  ADR acceptance own process/network/session authority and native evidence.

## Verification

Windows 11 local evidence on 2026-08-21:

- 10 runtime, 6 controller, 4 renderer, 33 D4, 32 UI-model, 5 Connection
  Library, and 46 command-palette tests passed;
- the tests cover no-scan-on-open, explicit selection/review/confirm, grant
  revocation, stale generations, publish-before-wake, joined shutdown,
  last-known-good errors, redacted recovery, metadata CAS success/conflict,
  read-only recent, disabled authority, focus, keyboard/pointer/IME, modal
  stacking, hostile text, all five filters, and bounded extreme geometry;
- `cargo deny check` passed advisories, bans, licenses, and sources; dependency
  inversion shows `rfd` as the terminal's only new direct dependency and
  `pollster` as its only new transitive package;
- the release 10,000-record Criterion projection measured 7.1790–7.7931 ms
  against the below-16-ms target (the earlier same-host baseline was
  7.0513–7.4408 ms; no statistical regression claim is made from one run); and
- the release executable is 22,670,336 bytes, 650,752 bytes (2.96%) above the
  22,019,584-byte same-host pre-M1 baseline.

The final evidence ladder additionally requires Clippy, repository and
architecture validation, documentation/assurance checks, Rustfmt, diff checks,
and `cargo ready`; their final results are recorded in `docs/TESTING.md` and the
change handoff. Native macOS/Linux picker and static-permission/recovery runs,
plus controlled Narrator/NVDA, VoiceOver, and Orca verification, remain external
release evidence. Model, cross-compile, or Windows results are not reported as
those native proofs.
## Consequences

### Positive

- Users gain a safe product-visible inventory browser and public metadata edits
  without granting connection or credential authority.
- One service owns stores, generations, cancellation, wake publication, and
  teardown; renderer and window controllers remain thin consumers.
- Native dialogs preserve familiar platform file-selection UX and sandbox/
  portal behavior while D4 remains the exact grant authority.
- Removing or disabling the Hub leaves ordinary terminal and manual OpenSSH
  behavior unchanged.

### Trade-offs

- Linux/BSD packaging must provide a supported XDG portal backend and documented
  fallback dependency; otherwise selection is unavailable and fails closed.
- Users reselect files after restart because M1 deliberately persists no grant
  paths.
- The picker adds a runtime dependency and platform code that must remain
  pinned, audited, and measured.
- Native accessibility and visual claims remain limited to the exact controlled
  systems actually exercised.
