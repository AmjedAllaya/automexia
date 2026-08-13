# Ghostty keyboard compatibility

This document describes Automexia's **currently implemented default shortcut
subset**. It is not a selectable or exact Ghostty configuration profile:
Automexia does not yet implement Ghostty's full trigger/action language,
versioned profile fixtures, sequences, key tables, consumption rules, or every
action. Those decisions and their implementation order are tracked in the
[full Ghostty compatibility roadmap](GHOSTTY-COMPATIBILITY-ROADMAP.md).

Automexia mirrors every audited Ghostty default whose action exists in the
Automexia frontend. The current hand-maintained baseline is Ghostty commit
[`d2c70a8c7b9b6893c13640c02d7b6f9a1624f3f0`](https://github.com/ghostty-org/ghostty/blob/d2c70a8c7b9b6893c13640c02d7b6f9a1624f3f0/src/config/Config.zig#L6504-L7272),
audited on 2026-08-13. Ghostty documents
`ghostty +list-keybinds --default` as the authoritative installed-version
listing; its [keybinding guide](https://ghostty.org/docs/config/keybind) and
[action reference](https://ghostty.org/docs/config/keybind/reference) explain
trigger and action semantics.

`Cmd` means the Command key on macOS. `Super` means the Windows key on Windows
and the desktop's Super/Meta key on Linux or BSD. User bindings in
`config.toml` continue to override these defaults.

## Windows, Linux, and BSD

| Shortcut | Automexia action |
|---|---|
| `Ctrl`+`Shift`+`,` | reload configuration |
| `Ctrl`+`,` | open Automexia settings |
| dedicated Copy key / `Ctrl`+`Insert` / `Ctrl`+`Shift`+`C` | copy selection |
| dedicated Paste key / `Ctrl`+`Shift`+`V` | paste clipboard |
| `Shift`+`Insert` | paste selection clipboard |
| `Ctrl`+`=` or `Ctrl`+`+` | increase font size |
| `Ctrl`+`-` | decrease font size |
| `Ctrl`+`0` | reset font size |
| `Ctrl`+`Shift`+`N` | new OS window |
| `Alt`+`F4` | close current OS window |
| `Ctrl`+`Shift`+`Q` | quit Automexia |
| `Ctrl`+`Shift`+`T` | new window-level tab |
| `Ctrl`+`Shift`+`W` | close current window-level tab |
| `Ctrl`+`Tab` / `Ctrl`+`Shift`+`Tab` | next / previous tab |
| `Ctrl`+`Shift`+`Right` / `Ctrl`+`Shift`+`Left` | next / previous tab |
| `Ctrl`+`PageDown` / `Ctrl`+`PageUp` | next / previous tab |
| `Ctrl`+`Shift`+`PageDown` / `Ctrl`+`Shift`+`PageUp` | move tab next / previous |
| `Alt`+`1` … `Alt`+`8` | select tab 1 … 8 |
| `Alt`+`9` | select last tab |
| `Ctrl`+`Shift`+`O` | split right |
| `Ctrl`+`Shift`+`E` | split down |
| `Ctrl`+`Super`+`]` / `Ctrl`+`Super`+`[` | next / previous split |
| `Ctrl`+`Shift`+`Super`+Arrow | resize split in the arrow direction |
| `Shift`+`Home` / `Shift`+`End` | scroll to top / bottom |
| `Shift`+`PageUp` / `Shift`+`PageDown` | scroll one page up / down |
| `Ctrl`+`Shift`+`Up` / `Ctrl`+`Shift`+`Down` | previous / next semantic prompt |
| `Ctrl`+`Shift`+`F` | start terminal search |
| `Escape` while searching | end terminal search |
| `Ctrl`+`Shift`+`A` | select all |
| `Ctrl`+`Enter` | toggle fullscreen |
| `Ctrl`+`Shift`+`P` | open command palette |

Ghostty currently registers both `close_surface` and `close_tab:this` on
`Ctrl`+`Shift`+`W`; the later `close_tab:this` registration is the effective
default and is what Automexia implements.

## macOS

| Shortcut | Automexia action |
|---|---|
| `Cmd`+`Shift`+`,` | reload configuration |
| `Cmd`+`,` | open Automexia settings |
| dedicated Copy key / `Cmd`+`C` | copy selection |
| dedicated Paste key / `Cmd`+`V` | paste clipboard |
| `Cmd`+`Shift`+`V` | paste selection clipboard |
| `Cmd`+`=` or `Cmd`+`+` | increase font size |
| `Cmd`+`-` | decrease font size |
| `Cmd`+`0` | reset font size |
| `Cmd`+`K` | clear screen and saved history |
| `Cmd`+`A` | select all |
| `Cmd`+`Q` | quit Automexia |
| `Cmd`+`N` | new OS window |
| `Cmd`+`T` | new window-level tab |
| `Cmd`+`W` | close focused split, or its tab when no split remains |
| `Cmd`+`Alt`+`W` | close current window-level tab |
| `Cmd`+`Shift`+`W` | close current OS window |
| `Ctrl`+`Tab` / `Ctrl`+`Shift`+`Tab` | next / previous tab |
| `Cmd`+`Shift`+`]` / `Cmd`+`Shift`+`[` | next / previous tab |
| `Cmd`+`1` … `Cmd`+`8` | select tab 1 … 8 |
| `Cmd`+`9` | select last tab |
| `Cmd`+`D` | split right |
| `Cmd`+`Shift`+`D` | split down |
| `Cmd`+`]` / `Cmd`+`[` | next / previous split |
| `Cmd`+`Ctrl`+Arrow | resize split in the arrow direction |
| `Cmd`+`Home` / `Cmd`+`End` | scroll to top / bottom |
| `Cmd`+`PageUp` / `Cmd`+`PageDown` | scroll one page up / down |
| `Cmd`+`Shift`+`Up` / `Cmd`+`Shift`+`Down` | previous / next semantic prompt |
| `Cmd`+`Up` / `Cmd`+`Down` | previous / next semantic prompt |
| `Cmd`+`F` | start terminal search |
| `Cmd`+`Shift`+`F` or `Escape` while searching | end terminal search |
| `Cmd`+`G` / `Cmd`+`Shift`+`G` while searching | next / previous result |
| `Cmd`+`Enter` or `Ctrl`+`Cmd`+`F` | toggle fullscreen |
| `Cmd`+`Shift`+`P` | open command palette |
| `Cmd`+`Right` / `Cmd`+`Left` | send end-of-line / beginning-of-line |
| `Cmd`+`Backspace` | send delete-to-line-start |
| `Alt`+`Right` / `Alt`+`Left` | send next-word / previous-word |

## Collision-free Automexia extensions

Automexia keeps product-specific features on chords not occupied by the pinned
Ghostty defaults:

| Shortcut | Platform | Action |
|---|---|---|
| `Ctrl`+`T` | Windows/Linux/BSD | additional window-level tab alias |
| `Ctrl`+`Alt`+`T` | Windows/Linux/BSD | new independent tab in the selected pane |
| `Cmd`+`Alt`+`T` | macOS | new independent tab in the selected pane |
| `Ctrl`+`Alt`+`R` / `Ctrl`+`Alt`+`D` | all | clone active session right / down |
| `Ctrl`+`Alt`+`O` | all | open link-hint mode |
| `Ctrl`+`Shift`+`Space` | Windows/Linux/BSD | toggle vi mode |
| `Ctrl`+`Alt`+`Space` | Windows/Linux/BSD | toggle quake window |
| `Alt`+`Shift`+`T` | Windows/Linux/BSD | toggle appearance |
| `Ctrl`+`Shift`+`K` | Windows/Linux/BSD | clear screen and all scrollback |
| `F11` or `Alt`+`Enter` | Windows/Linux/BSD | additional fullscreen aliases |

Bare `Ctrl`+`R` and `Ctrl`+`D` are deliberately shell-owned, matching Ghostty:
PowerShell/Readline history search and shell EOF/logout work without terminal
remapping.

## Ghostty defaults not implemented

These shortcuts are intentionally unbound so they continue to reach the shell.
Automexia does not yet have the corresponding action:

| Ghostty shortcut | Missing Automexia feature |
|---|---|
| `Ctrl`+`Shift`+`J` / `Cmd`+`Shift`+`J` | write the visible screen to a temporary file and paste its path |
| `Ctrl`+`Shift`+`Alt`+`J` / `Cmd`+`Shift`+`Alt`+`J` | write the visible screen to a temporary file and open it |
| `Ctrl`+`Shift`+`Super`+`J` / `Cmd`+`Ctrl`+`Shift`+`J` | write the visible screen to a temporary file and copy its path |
| `Shift`+Arrow | extend or shrink the terminal selection by direction |
| macOS `Shift`+`Home`/`End`/`PageUp`/`PageDown` | extend the terminal selection by screen boundary or page |
| `Ctrl`+`Alt`+Arrow / macOS `Cmd`+`Alt`+Arrow | focus a split by geometric direction |
| `Ctrl`+`Shift`+`Enter` / macOS `Cmd`+`Shift`+`Enter` | zoom or restore the focused split |
| `Ctrl`+`Shift`+`I` / macOS `Cmd`+`Alt`+`I` | terminal inspector |
| macOS `Cmd`+`Ctrl`+`=` | equalize all split sizes |
| macOS `Cmd`+`J` | scroll to the current selection |
| macOS `Cmd`+`E` | search for the selected text |
| macOS `Cmd`+`Shift`+`T` or `Cmd`+`Z` | undo a recently closed window, tab, or split |
| macOS `Cmd`+`Shift`+`Z` | redo a recently undone surface operation |

Ghostty also declares `Cmd`+`Alt`+`Shift`+`W` for `close_all_windows`, but its
official action reference marks that action deprecated and ineffective on
macOS and Linux. Automexia does not copy that no-op binding.

The compatibility roadmap additionally tracks infrastructure that is not a
single shortcut: a typed compiled registry, named/versioned profiles, explicit
unbind layers, atomic last-known-good reload, action outcomes and fallthrough,
multi-key sequences, key tables, action chains, generated manifests/docs,
inspection CLI, migration tooling, fuzzing, and latency gates. None of those
capabilities should be inferred from the default tables above.

## Verification policy

Binding tests construct both platform tables on every host, reject overlapping
triggers inside each Ghostty table, and reject overlaps between Ghostty
shortcuts and inherited/common Automexia bindings. Mode-disjoint compound
bindings, such as copy followed by selection cleanup in vi mode, are tested as
intentional behavior rather than accidental duplicates. The command palette
also rejects duplicate non-empty shortcut labels.

When Ghostty changes its defaults, update the pinned commit, both platform
constructors, this document, command-palette labels, and the collision tests in
one pull request. Once generated profile fixtures and registry-derived docs are
implemented, they replace this manual synchronization process; until then this
matrix is canonical for shipped defaults only.
