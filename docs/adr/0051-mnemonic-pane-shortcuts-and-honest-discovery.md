# ADR 0051: Mnemonic pane shortcuts and honest command discovery

Status: Accepted for current source; native desktop evidence remains external

## Decision and ownership

Windows/Linux/BSD use Alt+R / Alt+D to clone right/down. Adding Shift creates a
fresh default-shell pane. R means Right, D means Down, Shift means Fresh.
The previous punctuation defaults are removed, not retained as hidden aliases.
Normal shell Ctrl+R/D remain untouched. Search, Vi and alternate-screen modes
retain the four new pane chords. Explicit user bindings win without migration.
macOS and the pinned Ghostty profile keep their existing platform contracts.

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

Alt+D and Alt+R are shell editing shortcuts; the user explicitly approved taking
them for pane creation after that tradeoff was explained. See
[PSReadLine's functions](https://learn.microsoft.com/en-us/powershell/module/psreadline/about/about_psreadline_functions).
Ctrl+Alt alternatives were rejected because of AltGr/layout ambiguity; punctuation
was rejected for discoverability. On macOS, Option participates in text entry and
Option-Command-D toggles the Dock, so existing Command-based bindings remain.
See [Apple's shortcut reference](https://support.apple.com/en-us/102650).

| Family reviewed | Result and reason |
|---|---|
| Fresh / cloned panes | Change only Windows/Linux/BSD to directional R/D with Shift for fresh; keep independent PTY semantics. |
| Windows, tabs, close and reorder | Retain conventional N/T/W, Tab, Page and digit controls; distinguish window tabs from local tabs. Never add an easy accidental Quit chord. |
| Pane focus, cycling and resizing | Retain arrows and F6; Linux's extra resize modifier avoids changing desktop/window-manager policy. |
| Copy, paste and selection | Retain platform conventions and selection-aware Ctrl+C; preserve shell interrupt and Windows paste. Prefer ordinary chords in labels. |
| Find, global search and command history | Retain F, Shift for broader search, previous/next Enter controls and command-jump arrows. Do not advertise search-only Shift+Enter as a global launch. |
| Fonts, zoom, appearance and fullscreen | Retain conventional +/-/0, L for font List and T for Theme. Linux actions with no default show Unbound, not Windows-only guesses. |
| Settings and application launchers | Retain existing mnemonics and mode restrictions; show the actual configured/platform key. |
| Palette categories and child lists | Preserve arrow/Tab navigation, explicit Enter, fixed left-arrow Back, Alt+Left, empty-query Backspace and Esc. |
| Search, Vi, image browsing and modal mnemonics | Preserve scoped owners and visible prompts; no new global single-letter shortcut competes with text input. |
| Explicit bindings and compatibility profiles | Preserve user priority, tombstones, conditional shadowing and strict-profile isolation; never rewrite preferences based on guessed provenance. |

This prioritizes familiar conventions and truthful discovery over giving every
infrequent action another global chord. It follows
[W3C keyboard guidance](https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/)
on predictable focus and shortcut conflicts; this is not a native accessibility claim.

## Verification and recovery

Regression tests first reproduced the old Back icon, old pane actions and
non-pane override-label failure. Full-table tests cover configured and isolated
Windows/Linux/BSD/macOS defaults, all four chords, retired punctuation, disabled
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
