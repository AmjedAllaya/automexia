# Ghostty keyboard compatibility

Automexia provides an explicit, versioned Ghostty 1.3 compatibility profile
without changing existing users' shortcuts. `automexia` remains the default.
The pinned profile is derived from Ghostty 1.3.1 tag `v1.3.1`, commit
`22efb0be2bbea73e5339f5426fa3b20edabcaa11`.

## Current status

| Area | Status | Result |
|---|---|---|
| Linux/BSD Ghostty 1.3 profile | **Fully done locally** | 72 normalized bindings and 85 upstream actions are generated from the reviewed 1.3.1 binary and checksum-verified offline. Native release smoke evidence is still required before an exact release claim. |
| Windows profile | **Fully done as an Automexia adaptation** | Deterministic Linux-to-Windows transform; it is not described as an upstream Ghostty Windows profile. Super uses the Windows key, primary selection uses the clipboard, and globals/intercepted combinations are diagnostic. |
| macOS profile | **Partially done** | The schema and fail-closed selector exist, but Automexia refuses the profile until a native macOS 1.3.1 fixture is generated and reviewed. It is never synthesized from Linux. |
| Typed registry, profiles, reload, sequences, tables, chains | **Fully done locally** | Bounded pure registry, direct/reverse allocation-free lookup, exact prefix flushing, per-surface state, strict/permissive diagnostics, atomic last-known-good reload, and OS-global ownership. |
| Stateless action families | **Fully done locally** | Finite fractional font sizing, absolute/line/fractional scrolling, clear variants, extended selection/search, inherited independent splits, logical-pixel resize, zoom/equalize, and bounded private screen/selection/scrollback export. |
| Tooling and release assurance | **Partially done** | CLI, migration, generation, verification, generated references, fuzz targets, properties, and Windows Criterion evidence exist. Native three-platform keyboard/visual/resource/assistive-technology and 30-day benchmark evidence remains a release gate. |
| Inspector and undo/redo | **Partially done** | The modal redacted inspector shows active/parked counts, supports newest restore and two-step clear, and parks complete closed top-level window tabs. Individual split, pane-local-tab, whole-native-window history, and controlled native lifecycle evidence remain gated. |

These limitations are why the project does not yet make an unqualified “full
Ghostty compatibility” release claim. See the
[phase roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md) for exact external gates.

## Select a profile

```toml
[keyboard]
binding-profile = "ghostty-1.3"
binding-strict = true
```

Profiles are:

- `automexia`: implicit, backward-compatible classic shortcuts;
- `ghostty-1.3`: pinned permanently to reviewed Ghostty 1.3.1 artifacts;
- `ghostty`: visible moving alias to the newest bundled verified profile.

The footer shows `GHOSTTY 1.3` for the pinned profile and `GHOSTTY 1.3 ↗` for
the moving alias. Selecting `ghostty` never rewrites the configuration. macOS
currently reports a clear unavailable-profile error and retains the last valid
registry.

To disable compatibility, set `binding-profile = "automexia"` and remove any
optional `[bindings].keybinds` entries. Reload is transactional: an invalid
candidate leaves every window on the last known-good profile and does not
recreate a PTY.

## Add or remove typed bindings

Typed lines are layered after the selected profile:

```toml
[bindings]
keybinds = [
  "ctrl+shift+t=new_tab:inherit",
  "ctrl+x>ctrl+s=write_screen_file:copy,plain",
  "nav/ctrl+h=goto_split:left",
  "ctrl+k=activate_key_table:nav",
  "ctrl+u=unbind",
]
```

Supported syntax includes logical Unicode keys, `physical:KeyA`, named keys,
modifiers, multi-key `>` sequences, `table/trigger`, `catch_all`, ordered
`chain=...` lines, and the `performable:`, `unconsumed:`, `all:`, and tightly
restricted `global:` prefixes. Limits are 4 trigger atoms per chord, 8 chords
per sequence, 32 actions per chain, 32 active tables, 4,096 bindings, 256
diagnostics, 128-byte identifiers, 4 KiB parameters, and 4 KiB retained prefix
bytes. Invalid or oversized input fails closed.

A typed Automexia-profile `unbind` suppresses the exact lower classic shortcut
and forwards the normal encoded key to the PTY. A later exact bind restores an
application owner. Strict Ghostty mode does not inject Automexia clone or
pane-local-tab shortcuts; those actions remain available from the palette.

## Discover effective actions and shortcuts

These commands run before GUI initialization and do not start Ghostty:

```text
automexia --list-actions --aliases --unavailable
automexia --list-keybinds --profile ghostty-1.3 --platform windows
automexia --list-keybinds --effective --shadowing --json
automexia --list-keybinds --profile ghostty-1.3 --explain ctrl+shift+t
```

`--platform` accepts `linux-bsd`, `macos`, or `windows`; `--origin` filters one
layer. JSON includes the schema/profile identity, trigger, action, table,
predicate, scope, origin, priority, policy, diagnostics when requested, and
registry statistics. The command palette reads the same registry, shows the
active trigger and origin, and labels profile actions that are absent as
`Unbound`.

Generated references are canonical for the bundled registry:

- [Ghostty 1.3 actions](generated/ghostty-1.3-actions.md)
- [Ghostty 1.3 Linux/BSD and Windows-adapted bindings](generated/ghostty-1.3-keybindings.md)

## Migrate only Ghostty keybindings

Preview is the default and never writes:

```text
automexia migrate ghostty --input PATH --dry-run
automexia migrate ghostty --input PATH --json
automexia migrate ghostty --input PATH --output PATH --apply --confirm
```

Migration reads only bounded `keybind` and `config-file` directives. It rejects
links, canonical include cycles, more than 32 files, include depth over 8, more
than 4 MiB aggregate input, files over 1 MiB, and more than 4,096 lines. It
never evaluates a shell, command, environment expression, provider, or Ghostty
process. The report preserves nearby comments and classifies exact,
translated, unsupported, and unsafe entries. Apply refuses to overwrite an
existing typed section, validates the resulting Automexia configuration,
creates a recoverable backup, and publishes atomically only after `--confirm`.

## Exact interaction behavior

- Bare `Ctrl+T`, `Ctrl+R`, and `Ctrl+D` reach the shell in strict Ghostty mode.
- `Ctrl+Shift+T` creates a window-level tab.
- `Ctrl+Shift+O` / `Ctrl+Shift+E` create right/down splits that inherit launch
  context but own independent PTYs.
- `Ctrl+Alt+Arrow` focuses geometrically; `Ctrl+Shift+Enter` toggles split zoom.
- Pending sequences are isolated per surface. Invalid continuations and
  registry replacement flush every original byte in order.
- Global bindings are accepted only for the OS-owned quick-terminal toggle;
  they never enter focused terminal dispatch.
- Font-size parameters accept finite fractional points. Automexia deliberately
  adapts Ghostty's broad range to the renderer's reviewed 6–100 point range;
  increase, decrease, and absolute set operations clamp without overflow.
- `scroll_to_row` numbers the complete buffer from its oldest row. Positive
  line and fractional-page values move downward, matching Ghostty even though
  Rio's internal display-offset sign is reversed. Integer and float conversion
  is saturating and scrolling remains an O(1), allocation-free grid operation.
- Ghostty `clear_screen` clears the primary screen and scrollback and falls
  through on the alternate screen. Automexia also exposes explicit
  visible-only, history-only, and combined actions.
- Screen exports are at most 4 MiB, private, collision-safe, normalized, never
  executed, and cleaned by count (16), age, and application shutdown. Selection
  export counts base and combining text before allocating the copied string.

The footer indicates the profile, pending sequence/table, diagnostic count, and
split zoom without depending on color alone.

## Inspector and topology history

The typed `inspector` action opens a renderer-owned modal containing only grid
size, viewport/history counts, terminal/keyboard modes, profile, redacted
binding origin/trigger, pending/table state, opaque active session/route IDs,
active-session count, at most eight diagnostic codes, and newest-first parked
summaries. A parked summary contains only its session count, aggregate history
lines, and remaining TTL. It excludes parked route IDs, titles, commands,
destinations and content, plus all environment values, clipboard contents,
working directories, credentials, and terminal output.

While the inspector is open:

- click **Restore newest** or press `R` to restore the newest eligible top-level
  tab;
- click **Clear parked** or press `C` once to review the destructive action;
- click **Confirm clear**, press `C` again, or press `Enter` to end every parked
  group through the existing PTY owner;
- press `Escape` during confirmation to cancel it, or otherwise to close; and
- every other key press and release is consumed by the modal and never reaches
  the active PTY.

The pointer controls are hidden when no entry exists and on compact viewports;
the keyboard controls remain available. Closing the inspector cancels an
unfinished clear confirmation. Clearing parked entries cannot be undone.

`undo` and `redo` currently apply to a complete closed top-level window tab.
The existing `ContextGrid`—including its splits, pane-local tabs, route IDs, and
independent PTYs—is parked rather than relaunched. History is memory-only and
bounded to 8 entries, 5 minutes, and 250,000 retained history lines per window;
child exit, pressure, expiry, and shutdown destroy entries through the existing
PTY owner. A later topology mutation invalidates redo. Individual split,
pane-local-tab, and whole native-window closure is deliberately unbound until
its separate native lifecycle evidence exists.

See [ADR 0027](adr/0027-redacted-compatibility-inspector.md) and
[ADR 0028](adr/0028-bounded-parked-pty-topology-history.md).

## Provenance and external evidence

Normal startup, builds, and tests are offline and never execute Ghostty. The
maintainer-only generator is pinned to official Ghostty 1.3.1 source and binary
hashes; checked-in outputs are regenerated and byte-compared by `cargo xtask`.
Upstream source/build/packaging/license authority is documented in
[ADR 0026](adr/0026-versioned-ghostty-keybinding-profiles.md).

A compatibility-profile release still requires native Linux/BSD and macOS
fixtures/smoke runs, Windows/Linux/macOS keyboard-layout and rendered-frame
matrices, screen-reader evidence, resource-cycle evidence, packaging checks, and
a like-hardware 30-day benchmark baseline. Controlled runs use the strict
private evidence manifest documented in the testing guide; QA publishes only a
redacted summary. Synthetic platform tables and a Windows test run do not
replace those gates.
