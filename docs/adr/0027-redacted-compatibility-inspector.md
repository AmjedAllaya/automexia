# ADR 0027: Redacted renderer-owned compatibility inspector

- Status: accepted
- Date: 2026-08-23
- Owners: Automexia maintainers
- Related: ADR 0004, ADR 0013, ADR 0026

## Context

Ghostty compatibility needs an inspector that explains terminal dimensions,
active modes, profile resolution, binding origin, and pending key state without
turning diagnostics into a second terminal-output, process, or secret viewer.
Environment values, clipboard contents, commands, current directories, paths,
and hidden or historical terminal output can contain credentials. Pixel-only
inspection would also be difficult to test and inaccessible to non-renderer
consumers.

## Decision

The desktop renderer owns one passive compatibility-inspector modal per window.
Its input is an immutable, renderer-neutral `InspectorSnapshot` with a positive
field allowlist: route ID, opaque session ID, grid dimensions, viewport offset,
history line count, terminal/keyboard mode names, requested profile, last
binding trigger and origin, pending-byte count, active table name, and at most
eight redacted diagnostic codes.

The inspector never receives or reads environment values, clipboard data,
terminal cells/output, commands, working directories, paths, provider data,
credentials, or PTY byte streams. It owns no filesystem, process, network,
authentication, configuration, PTY, or persistence capability. Route/session
identity is numeric and opaque. Strings are bounded or derived from typed enums,
and renderer layout is viewport- and scale-bounded.

The modal is keyboard controlled through the typed `inspector` action. It is
part of the existing overlay stack, restores terminal focus when dismissed,
and never writes terminal input. The stable Ghostty profile binding and user
bindings are discoverable through the same compiled registry.

## Alternatives considered

- Show environment, command, path, or output samples. Rejected because those
  values can contain secrets and are unnecessary for binding diagnosis.
- Parse renderer pixels or terminal output on demand. Rejected because pixels
  and untrusted terminal content are not authoritative state.
- Run an external inspector process. Rejected because it adds process, IPC,
  disclosure, and lifecycle authority for a passive view.

## Verification

- A schema test rejects forbidden field names and freezes the positive allowlist.
- Geometry tests cover compact and 200% scale-equivalent viewports.
- The typed action schema, generated references, CLI action inventory, and
  profile fixtures share stable action identity.
- Renderer-neutral and Windows-native keyboard/focus checks are local gates.
  Native macOS/Linux rendered-frame and assistive-technology evidence remains
  an external release prerequisite.

## Consequences

The inspector is useful for local compatibility diagnosis without increasing
secret or terminal-output exposure. Future fields require this ADR's allowlist,
redaction tests, bounds, and an architecture review. A future accessibility
adapter may consume the same snapshot; it must not expand the data authority.