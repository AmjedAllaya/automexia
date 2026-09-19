# Focused table output

Open **Ctrl+Shift+P → Tools → View Table Output**, or use **Ctrl+Shift+F7**
with the Automexia profile. The shortcut is customizable: select the palette
command and press **F2**, or double-click its shortcut badge. Other keyboard
profiles can bind the `ViewTableOutput` action explicitly.

Select a complete table first to choose exactly which output to view. Without
a selection, Automexia looks for the most recent aligned table at or above the
current scroll position. This is a focused, read-only snapshot; the shell keeps
running underneath. Close and reopen the view to capture newer output.

| Control | Action |
|---|---|
| Left / Right | Pan horizontally by one terminal cell. |
| Up / Down; Page Up / Page Down | Scroll rows or a page. |
| Home / End | Reach the first / last horizontal position. |
| Ctrl+Home / Ctrl+End (Cmd also works) | Reach the first / last row and column. |
| Shift+wheel; horizontal trackpad gesture | Pan horizontally. |
| Wheel | Scroll vertically. |
| Horizontal scrollbar | Click or drag to pan. |
| Copy all; Ctrl+C or Cmd+C | Copy the snapshot text without decorative borders. |
| Tab / Shift+Tab; Enter | Move between Back and Copy, then activate. |
| Escape; Back arrow | Return to the terminal. |

Columns never wrap in the focused view. Subtle separators mark rows and shared
column gutters. Text is neutral: the viewer does not infer resource health,
readiness or success from table values. Original terminal colours remain intact
underneath. Tabs are expanded from their actual grid positions, not guessed
from a fixed tab width. Copying in the terminal retains its normal semantics.

The viewer neither reruns commands nor changes the terminal grid, selection,
cursor, search history or scrollback. Typing, paste, IME commits and file drops
are not forwarded to the shell while the view is open. Its snapshot stays only
in memory and is released on close or active-session replacement. Nothing is
saved, uploaded or fetched from a provider.

## Recognition and limits

Use aligned plain-text tables with at least two rows and two columns separated
by shared whitespace gutters of at least two cells. This includes many `ls`,
Kubernetes and container listings, but not every possible command format.
Quoted CSV, JSON, box-drawing schemas, single-space columns and arbitrary prose
are not parsed as structured data. Unsupported output remains available in the
ordinary terminal. Alternate-screen and mouse-reporting applications are not
captured.

Capture is bounded to 256 KiB, 256 logical rows, 4,096 cells per row and 64
recognized columns. Automatic discovery inspects at most 4,096 native rows and
256 Ki native cells near the viewport, not unlimited history. Select a smaller
complete table if a limit is exceeded. A selection beginning halfway through a
row or a command that already truncated or hard-wrapped its output may not form
a recognizable table; the view cannot reconstruct missing application data.

Very small windows may have no room for table cells or labels; Escape remains
available, and enlarging the window restores the snapshot. Native compositor,
physical trackpad and assistive-technology coverage is separate from automated
model, input and controlled CPU-rendering tests; see [testing](../TESTING.md).
