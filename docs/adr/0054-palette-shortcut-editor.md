# ADR 0054: Palette-owned shortcut editing and application-owned persistence

Status: Accepted for current source; native desktop and AT evidence remain external

## Decision

Core bindings own command identity, candidate validation and overlay composition.
The existing command palette owns double-click/F2 recording, review and navigation.
The application is the sole publication and persistence owner. Universal keyboard
behavior does not belong in an extension; no new extension, dependency, dispatcher,
worker, protocol or global hotkey hook is needed. Existing mappings were moved to
the binding owner, not copied. A capability-free, hidden public library module
shares catalog identity and record validation with the application binary and
preference serializer; it has real consumers on both sides of that crate boundary.

The schema-1 UI preference record gains an optional `shortcuts` array, capped at
64 records inside the existing 16 KiB private TOML document. Old documents read
unchanged. Records contain a known catalog action and a validated logical/named
single trigger; no executable, shell bytes, path or credential is accepted. The
effective config carries this overlay in a non-deserializable field, keeping one
composition owner. Hand-edited config and pinned profile fixtures are unchanged.

Application validation rejects duplicate actions/triggers, explicit user text or
disabled-key collisions, typed prefixes, global/physical conflicts and advanced
chains/scopes. A simple action override removes its normal-mode aliases and adds
one classic binding, excluding Search, Vi and alternate screen. Existing classic
and typed aliases outside normal mode are retained as disjoint predicates, so
combined modes cannot execute duplicate copies. The configured
base remains authoritative for conflict checks: moving one UI shortcut does not
implicitly free its configured key for another command. Reset removes one record.

Save updates only prepared bindings, not fonts, PTYs or pane geometry. Existing
key-sequence cancellation and binding publication stay with Screen. A bounded
application event carries no untrusted payload; the palette owns the pending
typed edit until the application takes it. Reload cancels queued edits. Other
windows receive the same binding snapshot; obsolete UI receipts are invalidated.
No filesystem work occurs on keyboard, mouse or rendering paths.

The existing coalescing atomic preference writer now returns monotonically tagged
receipts and wakes the event loop after publication. An older completion cannot
certify a newer save. Error notification consumption does not clear durable failure
state. Back after Save does not roll back submitted work. Independent processes
retain the existing lock/last-writer-wins snapshot semantics; this is not a new
cross-process merge service.

On incompatible startup overlays, use configured bindings with a warning and keep
stored records recoverable. Invalid live reloads retain the last-known-good config.
Advanced mappings, modifier-only/dead keys, reserved dialog keys, raw text and
shell Ctrl+C/D/R/Z are not recorded as simple UI shortcuts. Users retain explicit
advanced configuration control. External interception cannot be detected reliably.

## Interaction and alternatives

Double-clicking a badge and F2 on a selected command enter the same editor.
Save/Reset/Cancel share keyboard and pointer activation. Query and selection stay
in the existing palette; repeated Enter cannot run a command through a navigation
boundary. IME, paste, drops and wheel input remain modal. Focus loss pauses draft
capture. Drawing and hit-testing share live logical geometry; tiny viewports clip
content rather than escaping the surface. Existing branding and fonts are reused.

A text-file-only workflow is still available for advanced users, but is not enough
for discoverability. Inline editing every row would complicate navigation and
collision feedback. A dedicated short-lived recorder gives a clear Save boundary.
The interaction follows [VS Code keybinding editing](https://code.visualstudio.com/docs/configure/keybindings)
and [W3C modal keyboard guidance](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/),
without claiming native screen-reader conformance from a model summary.

## Evidence and rollback

Tests cover actual platform tables, strict profiles, tombstones, typed chains,
explicit text, invalid records, mode exclusion, repeat/cancel/reset/retry, queued
edit invalidation, tiny-to-8K geometry, durable failure and a real process restart.
The existing exact CPU label tests remain independent of native frames. The
record/validate/cancel benchmark exercises the owning model, not desktop latency.
See [shortcut editor assurance](../TESTING.md#shortcut-editor-assurance).

Rollback removes the relevant UI override record; the base config remains intact.
Native pointer routing, layouts, compositor pixels, IME and screen readers on each
supported OS remain separate release gates, not implied by unit-test success.
