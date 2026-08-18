# ADR 0010: Ghostty-compatible keyboard defaults

Status: Superseded by ADR 0011

## Context

Ghostty users should be able to move to Automexia without relearning routine
window, tab, split, search, scrolling, clipboard, and font controls. Automexia
also has pane-local tabs and independent session clones that Ghostty does not
model with equivalent default actions. Earlier Automexia defaults used bare
`Ctrl+R` and `Ctrl+D` for cloning, which displaced PowerShell/Readline history
search and shell EOF/logout.

## Decision

The default tables mirror the effective Ghostty defaults at pinned commit
`d2c70a8c7b9b6893c13640c02d7b6f9a1624f3f0` separately for macOS and
non-macOS platforms. Only actions supported by Automexia are installed.
Unsupported actions remain unbound and are listed in
`docs/GHOSTTY-KEYBOARD-COMPATIBILITY.md`.

Automexia-specific actions use collision-free extensions:

- `Ctrl+Alt+T` or `Cmd+Alt+T` creates a pane-local tab;
- `Ctrl+Alt+R` and `Ctrl+Alt+D` clone the active session;
- `Ctrl+Alt+O` opens link hints;
- bare `Ctrl+R` and `Ctrl+D` remain shell-owned.

User configuration remains authoritative. A user binding replaces a matching
default trigger and mode scope.

Both platform tables are compiled by host-independent unit tests. Tests reject
overlaps inside each table and overlaps with inherited common bindings.
Intentional inherited compound actions are not treated as accidental
duplicates. Command-palette shortcut labels have a separate uniqueness test.

## Consequences

Ghostty users receive familiar defaults, Automexia's extra scopes remain
discoverable, and shell editing controls work without remapping. Adding a
Ghostty-compatible binding now requires an available Automexia action, action
dispatch coverage, a collision test, a palette-label update when applicable,
and an update to the compatibility matrix. Advancing the pinned Ghostty commit
must be a reviewed compatibility change rather than an undocumented drift.

This decision covers only the current fixed default tables. It does not claim
an exact Ghostty profile or approve a public compatibility language. The
[full compatibility roadmap](../roadmap.md) proposes a
private typed registry, versioned opt-in profiles, atomic reload, sequences,
tables, generated manifests, and missing actions. The dependency boundary,
configuration schema, and dispatch semantics require a new ADR before that
architecture supersedes this one. Until then, `automexia` remains the only
implicit profile and compatibility is never enabled or migrated silently.
