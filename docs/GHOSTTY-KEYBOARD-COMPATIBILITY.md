# Ghostty keyboard compatibility

Automexia does not use Ghostty shortcuts as its implicit default. The original
Automexia platform mappings were restored by ADR 0011 after user evaluation.
The active shortcuts are documented in [configuration](CONFIGURATION.md) and
are enforced by binding, palette-label, override, and architecture tests.

Ghostty compatibility remains a future, explicit opt-in profile. Enabling such
a profile must never silently rewrite an existing configuration or replace the
`automexia` default. The implementation must provide a typed binding registry,
versioned upstream fixtures, atomic reload, collision diagnostics, generated
documentation, and clear handling for unsupported Ghostty actions before the
profile can be advertised.

The audited comparison baseline remains Ghostty commit
[`d2c70a8c7b9b6893c13640c02d7b6f9a1624f3f0`](https://github.com/ghostty-org/ghostty/tree/d2c70a8c7b9b6893c13640c02d7b6f9a1624f3f0),
but no binding from that baseline is automatically installed merely because it
exists in Ghostty. The complete implementation and verification sequence is in
the [Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md).

## Deliberate classic differences

| Automexia default | Action | Strict Ghostty profile behavior, if implemented |
|---|---|---|
| `Ctrl`+`T` | new window-level tab | forwarded to the shell |
| `Ctrl`+`Shift`+`T` | new pane-local independent tab | new window-level tab |
| `Ctrl`+`Shift`+`R` / `Ctrl`+`Shift`+`D` | fresh right/down split | Ghostty uses `Ctrl`+`Shift`+`O` / `Ctrl`+`Shift`+`E` |
| `Ctrl`+`R` / `Ctrl`+`D` | clone active session right/down | forwarded to history search / EOF |
| `Ctrl`+`Alt`+`R` / `Ctrl`+`Alt`+`D` | explicit history-search / EOF passthrough | available for profile-specific actions |
| `Alt`+Arrow | focus the nearest pane geometrically | geometric split focus uses profile-specific chords |
| `F6` / `Shift`+`F6` | cycle panes in visual order | available as configurable split actions |
| `Alt`+`PageDown` / `Alt`+`PageUp` | next/previous tab inside the selected pane | Ghostty has no Automexia pane-local-tab scope |
| `Ctrl`+`Tab` / `Ctrl`+`Shift`+`Tab` | next/previous window-level tab | next/previous tab |

macOS retains the classic `Cmd`+`T` window tab, `Cmd`+`Shift`+`T` pane-local
tab, and `Cmd`+`D` / `Cmd`+`Shift`+`D` fresh splits. Session cloning remains on
`Ctrl`+`R` / `Ctrl`+`D` on every platform. It uses `Cmd`+`Alt`+Arrow for
geometric pane focus and `Cmd`+`Alt`+`]` / `Cmd`+`Alt`+`[` for pane-local tab
navigation.

## Policy

- `automexia` remains the only implicit profile for v0.4.
- A future Ghostty profile is selected explicitly and is versioned.
- User bindings remain authoritative over any profile defaults.
- Normal startup, builds, and tests never execute Ghostty or access its files.
- Compatibility claims require generated fixtures and host-independent tests.
- Missing Ghostty actions stay unbound; they are never mapped to an unrelated
  Automexia action merely to increase shortcut counts.
