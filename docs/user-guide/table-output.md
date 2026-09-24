# Table output

Header tables in normal terminal output can appear inline with borders. Values
wrap inside their own cells when a pane narrows, and every cell in a row keeps
the same height. Long names wrap between complete graphemes; spaces provide
preferred breaks in values such as `94 (17s ago)`. The terminal's original text,
VT grid, cursor and reflow remain the source of truth.

Automatic presentation recognizes aligned whitespace tables, Markdown pipe
tables, and ASCII or Unicode table frames. Headers may use uppercase, mixed-case
or lowercase labels when a ruler or typed data makes their role clear. Empty
cells and explicitly ruled single-column tables are supported. Recognition uses
the output structure rather than command names, and does not derive resource
health from a value. Unsupported output
keeps its ordinary terminal presentation.

Borders and glyphs stay inside their pane and cell bounds. Source terminal
colours and the selection-foreground preference remain authoritative. Hovered
links retain their underline in the active pane. Pointer selection maps displayed characters back
to the original terminal cells; copying does not include decorative borders or
insert the presentation's extra line breaks. This presentation does not run
commands, change clipboard contents, or intercept normal shell input.

## Focused table viewer

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

A header and at least one data row are required. Whitespace tables normally use
shared gutters of at least two cells; a single-space layout needs stronger
header evidence. Explicit rulers also support a single column. Ambiguous lists,
quoted CSV, JSON and arbitrary prose are not parsed as structured data. Producer
hard line breaks and truncated values cannot be reconstructed. Unsupported output remains available in the
ordinary terminal. Alternate-screen and mouse-reporting applications are not
captured. Inline presentation is also disabled while terminal vi mode is active.
Output with images or unsupported text decorations retains its ordinary rendering.

Each recognized table is bounded to 256 KiB, 256 logical rows, 4,096 cells per row
and 64 columns. Inline discovery examines at most 512 native rows and 64 Ki
native cells near the viewport, retaining at most four table surfaces and trying
at most eight header starts per contiguous block. Wrapping
is bounded to 4,096 content lines and 32,768 fragments per table. A pane must fit
at least one complete grapheme and the cell padding in every column; if it
cannot, or a limit is exceeded, the original terminal output remains available.
Enlarging the pane allows a supported inline layout to return.

Opening the focused viewer without a selection discovers candidates within at
most 4,096 native rows and 256 Ki native cells. Select a smaller complete table
if a capture limit is exceeded. A selection beginning halfway through a
row or a command that already truncated or hard-wrapped its output may not form
a recognizable table; the view cannot reconstruct missing application data.

Very small focused-view windows may have no room for cells or labels; Escape
remains available, and enlarging the window restores the snapshot. Model,
input and controlled CPU-rendering evidence is distinct from native compositor,
physical trackpad and assistive-technology coverage; see
[testing](../TESTING.md). No native desktop or screen-reader coverage is implied
by the common frontend code.
