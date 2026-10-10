# ADR 0051: Mnemonic pane shortcuts and honest command discovery

Status: Accepted for current source; native desktop evidence remains external

## Decision and ownership

Windows/Linux/BSD use Alt+J to clone right and Alt+Shift+J for a
fresh right pane. Alt+D clones down; Alt+Shift+D starts fresh down. The former
right-pane Alt+R defaults and earlier punctuation defaults are removed.
Normal shell Ctrl+R/D and Alt+E/R remain untouched. Search, Vi and alternate-screen
modes retain the pane chords. Explicit user bindings win without migration.
macOS and the pinned Ghostty profile keep their existing pane contracts.

Back uses an unframed left arrow on the existing vector grid in both the
persistent header and the navigation row. Its click target, Back label,
keyboard alternatives, focus restoration and non-executing ownership stay intact.

Core application bindings and palette presentation remain the owners. The
existing effective-binding label reconciliation now covers every palette action
at construction/reload, not at each input/frame. It distinguishes ClearHistory
from ClearScreen, search-only from global keys, and missing bindings from
unavailable commands. Ordinary copy/paste chords take precedence over uncommon
dedicated hardware keys. Typed profile/user labels retain priority; an inactive
mode, other scope or multi-action chain is not advertised as a simple direct key.
Typed start_search is labeled only as forward pane search, matching its actual
parameter-free dispatcher contract; it does not masquerade as backward/global search.
No new extension, dependency, worker, persistence or dispatcher is introduced.

## Tradeoffs and shortcut-family audit

The user chose preserving shell and OS keys over Alt+E: Fish uses Alt+E for
its external command editor, and macOS Option+E enters accents. The right-pane
Alt+J pair is unassigned in the reviewed stock Bash, Zsh, Fish and PowerShell
line editors, while Alt+R remains available for shell editing and external overlays.
Down-pane Alt+D remains the earlier explicit tradeoff; users can restore it with
ReceiveChar. No bare Ctrl+F9/F10 (KDE desktop actions), Shift+F10 (context menu),
Ctrl+Shift+E (input methods), Ctrl+Alt character (AltGr) or bare Option key is added.
External user/desktop bindings cannot be detected or guaranteed by an application.
References: [Fish](https://fishshell.com/docs/current/interactive),
[Apple](https://help.apple.com/pages/mac/4.3/help/English.lproj/pgs/sl151633f0.html),
[Windows](https://support.microsoft.com/en-us/windows/keyboard-shortcuts-in-windows-dcc61a57-8ff0-cffe-9796-cb9706c75eec),
[KDE](https://docs.kde.org/stable_kf6/en/khelpcenter/fundamentals/fundamentals.pdf).

| Family reviewed | Result and reason |
|---|---|
| Fresh / cloned panes | Use the Alt+J pair for right panes and retain down-pane/macOS bindings; keep independent PTY semantics. |
| Windows, tabs, close and reorder | Retain conventional N/T/W, Tab, Page and digit controls; distinguish window tabs from local tabs. Use deliberate Ctrl+Shift+Q, never bare Escape, for Quit. |
| Pane focus, cycling and resizing | Retain arrows and F6; Linux's extra resize modifier avoids changing desktop/window-manager policy. |
| Copy, paste and selection | Retain platform conventions and selection-aware Ctrl+C; preserve shell interrupt and Windows paste. Prefer ordinary chords in labels. |
| Find, global search and command history | Retain F, Shift for broader search, previous/next Enter controls and command-jump arrows. Do not advertise search-only Shift+Enter as a global launch. |
| Fonts, zoom, appearance and fullscreen | Retain conventional +/-/0, L for font List and T for Theme. Linux fullscreen now has its own F11 default. |
| Settings and application launchers | Retain existing mnemonics and mode restrictions; show the actual configured/platform key. |
| Palette categories and child lists | Preserve arrow/Tab navigation, explicit Enter, fixed left-arrow Back, Alt+Left, empty-query Backspace and Esc. |
| Search, Vi, image browsing and modal mnemonics | Preserve scoped owners and visible prompts; no new global single-letter shortcut competes with text input. |
| Explicit bindings and compatibility profiles | Preserve user priority, tombstones, conditional shadowing and strict-profile isolation; never rewrite preferences based on guessed provenance. |

This prioritizes familiar conventions and truthful discovery over giving every
infrequent action another global chord. It follows
[W3C keyboard guidance](https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/)
on predictable focus and shortcut conflicts; this is not a native accessibility claim.

## Complete classic discovery and concise labels

Current source assigns Ctrl+Shift+Q (Quit), Alt+Shift+B (backward pane search),
Ctrl+Alt+K (clear screen and history) on Windows/Linux/BSD, and F11 for Unix
fullscreen. macOS adds Cmd+Alt+K for clear screen/history and retains its other
platform defaults. F1/F2/F3/F5/F12 with Ctrl+Shift (Cmd+Shift on macOS) open
Workflow & Output,
Terminal Appearance, Themes, Profiles and Recovery through existing owners.
These five launchers exclude Search, Vi and alternate screen. All catalog
commands have direct classic defaults when the corresponding navigation/split
features are enabled. Quit and ClearScreen added
here exclude Search, Vi and alternate-screen modes; existing macOS Quit is kept.
Escape remains cancellation/back or terminal input, not destructive Quit.

Palette chips show only resolved keys. Source provenance remains registry and
inspector data. An explicitly unbound, shadowed or profile-absent action shows
Enter for the existing selected-row activation, never a guessed global key.
No user settings or pinned compatibility tables are migrated. Ctrl+Alt+K avoids
changing the established history-only binding; layouts with AltGr can use a
custom mapping or the palette. External overlays retain their own controls.

This keeps the existing core dispatcher, registry, shutdown and renderer owners;
no new authority, worker, dependency, allocation on key dispatch or persistence.
[W3C combobox guidance](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/)
informs Escape cancellation and Enter acceptance, without claiming native AT
certification. A complete-table test checks real bindings independently of
labels; literal CPU glyph pixels and geometry cover label output at 100–400%.
Native desktop/assistive-technology evidence remains external.

## Verification and recovery

Regression tests first reproduced the old Back icon, old pane actions and
non-pane override-label failure. Full-table tests cover configured and isolated
Windows/Linux/BSD/macOS defaults, all four pane chords, exact letter-key
modifiers, preserved Alt+E/R, retired punctuation, disabled
splits, modes, overrides, typed tombstones, chains and labels across reload.
The Automexia classic inventory is regenerated with its digest; pinned Ghostty
bindings are unchanged. Repository mutations reject removal of either platform's
chords or mode guards and reintroduction of retired aliases.
Arrow coordinates are checked independently at multiple scales; this is not
native GPU pixel evidence. Existing navigation and no-PTY tests remain.
Binding-label storage is bounded by the static catalog; no hot-path I/O is added.

Physical keyboard layouts, native frames and screen-reader delivery remain
external. Published packages are unchanged. Restore shell Alt editing with
ReceiveChar overrides or restore older split shortcuts explicitly; see
[Keyboard migration](../KEYBOARD.md#command-palette). A normal revert restores
the former defaults without changing saved configuration or sessions.
