# ADR 0049: Grouped command discovery and pane shortcuts

Status: Accepted for current source; native visual and accessibility evidence pending

The non-macOS pane chords and pane-only shortcut-label reconciliation below are
superseded by [ADR 0051](0051-mnemonic-pane-shortcuts-and-honest-discovery.md).
The category, navigation, independent-session and native-evidence contracts remain.

## Decision

The application-owned command palette opens on six categories. One exhaustive
mapping groups existing actions without creating another executor or command
catalog. Typing searches commands and category names globally. Clearing the
query restores the current category; Back restores the root category selection.
Both pointer and keyboard selection pass through one application activation
method. Category/Back rows have no command, clipboard or PTY authority. Empty
results remain editable, and repeated Enter cannot activate the first command
after opening a category. Query, selection and wheel state remain window-local.

Windows/Linux/BSD fresh splits use Alt+Shift+Plus/Minus; shifted and unshifted
logical punctuation are accepted. Alt+Shift+R/D clone the active session right
or down. macOS retains Cmd+D/Shift+D for fresh splits and uses Cmd+Alt+Shift+R/D for
clones. Bare Ctrl+R/D remain shell-owned. Explicit user bindings are preserved;
the pinned Ghostty profile is unchanged. This supersedes only ADR 0041's
unbound-clone decision, not its shell-control or independent-PTY boundaries.
Fresh-split and clone labels share the existing effective-binding owner. A
single typed lookup preserves profile/user precedence; disabled defaults and
shadowed punctuation must not leave stale advertised shortcuts.

## Rationale and alternatives

Browsing categories reduces the initial list without making users drill down
when they know a command name. One shallow level avoids a deep menu hierarchy.
The existing branded icons, fuzzy matching, viewport fitting and scrollbar
remain authoritative. A new extension, crate or web-widget dependency would
add ownership and runtime cost without fitting native application input.

The split punctuation follows a familiar
[Windows Terminal pattern](https://learn.microsoft.com/en-us/windows/terminal/panes).
Directional R/D mnemonics distinguish cloning from fresh splits. Reserving
Ctrl+R/D or replacing pane-focus arrows was rejected.
Mac clone chords include Shift to avoid the operating system's
[Option-Command-D Dock shortcut](https://support.apple.com/en-us/102650).
Query ownership, explicit
Enter activation and Escape cancellation follow the
[W3C combobox interaction guidance](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/)
as design guidance, not a claim that a native widget implements web ARIA.

## Verification and recovery

Tests cover every category/action, global search, Back, keyboard/pointer model
paths, wheel state, resizing, query limits, non-executable navigation, all
platform default tables, exact overrides and profile isolation. Native test
snapshots expose public scope/count semantics without query or external-item
content. These snapshots are not a screen-reader adapter or native pixel proof.
Physical keyboard layouts, compositor frames and assistive-technology checks
remain platform-specific release gates. Existing released binaries are unchanged.

Users can restore earlier fresh split keys explicitly; no configuration is
rewritten. Removing an override restores current defaults. Revert this change
normally to restore the previous palette without touching sessions or data.
