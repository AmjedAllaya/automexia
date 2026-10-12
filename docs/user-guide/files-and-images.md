# Files, output, and images

Automexia adds output styling and image previews. Your commands still control
their output. This guide explains what changes on screen and how to preview images.

## The basic rule: presentation must not change command output

Automexia can add:

- prompt/context rows;
- icon-aware interactive listings where shell integration supports them;
- semantic highlighting for common tool states;
- terminal graphics rendered from supported escape protocols;
- local image quick-look overlays;
- selection/search/footer UI.

These features are not supposed to rewrite the bytes that scripts consume. When an application emits its own ANSI styling, that styling remains authoritative over presentation enhancements.

## Prompt context and path

In integrated shells, Automexia displays context tags and a full path row. Your
shell controls the editable command line. Resizing can rearrange context without
changing shell input.

Use these labels for orientation. Scripts should read state from shell commands,
environment variables, or tools, rather than screen labels.

## Icon-aware listings

Interactive `ls`/`ll` presentation can add file/folder icons and stable category colors where the shell integration supports it. Long-listing variants can present structured columns while preserving native/script behavior boundaries.

Important practical rules:

- Use the enhanced aliases/wrappers for interactive browsing.
- Use native commands/explicit script forms when exact automation semantics matter.
- In CMD, built-in `dir` is deliberately not replaced.
- A user-defined `EZA_COLORS` is not overwritten by Automexia.

If icons are missing, first check [Troubleshooting](../TROUBLESHOOTING.md) rather than installing unrelated shell/profile modifications.

## Semantic output

Automexia may give common build, test, and structured-log warnings or errors a restrained presentation treatment without changing the underlying text.

This is visual only:

- copied text remains terminal text;
- explicit application ANSI colors win;
- selection and search highlights take higher priority;
- semantic recognition does not execute or alter commands.

Completed commands with proven shell boundaries also group their actual output
inside a restrained tinted band. A gap, end rule, and known success/failure plus
duration badge separate that result from the next command. This applies to any
output-producing command, including ordinary output, errors, pipelines, native
programs, and listings; it is not an `ls`-specific effect. A new live result
lightens once for 540 milliseconds and then keeps its persistent grouping.
Commands without output keep their truthful compact completion rule/status but
do not receive an empty output surface or borrow the preceding command's state.
Resizing and jumping between commands preserve one non-overlapping badge per
visible result; a prompt that also bounds the preceding output never displays
both commands' datetime or duration text in the same row. Optional prompt
context truncates before the reserved result-label region, so neither surface
can cover the other on narrow or historical rows.
The full grouping is available in integrated PowerShell, Bash, Zsh, and Fish
sessions. Integrated CMD also groups output, but uses neutral styling because
stock `cmd.exe` cannot provide a generic truthful exit status or duration.

Use it as a scanning aid, not as a replacement for the tool's own exit code or structured output.

## Three ways to work with images

| Approach | Use it when | Who controls display? |
|---|---|---|
| **Inline terminal graphics protocol** | A terminal-aware application wants to draw images in the terminal grid | The PTY application + terminal protocol |
| **Automexia local quick look** | A local image path is already visible in terminal output and you want a preview | Automexia overlay |
| **External trusted application** | The file is unsupported, remote, complex, or you need editing/full fidelity | Your chosen application |

## 1. Inline terminal graphics

**Available now.** Programs can explicitly send supported image escape sequences. Automexia supports these protocol families:

- Kitty Graphics;
- iTerm2 OSC 1337 inline images;
- Sixel.

Image-aware programs control placement, scrolling, size, and image lifetime.

Examples include `kitten icat`, `imgcat`, and `wezterm imgcat`. If a tool does not
recognize Automexia, select its Kitty, iTerm2, or Sixel adapter. Do not change
`TERM_PROGRAM` to impersonate another terminal.

Terminal multiplexers also need to pass the chosen graphics protocol through.

## 2. Local quick look

**Available now.** Quick look previews a local image whose path is visible in
terminal output. It opens an overlay without sending input to the shell.

### Mouse workflow

1. Move the pointer over a supported filename/path.
2. Hold it stable briefly; Automexia waits about 100 ms before decoding so fast pointer movement does not trigger constant file work.
3. The temporary preview appears if the path passes validation.
4. Click the path to pin the preview.
5. Click elsewhere or press `Esc` to dismiss.

### Keyboard workflow

1. Select the image path as terminal text.
2. Press `Ctrl+Alt+I` on Windows/Linux/BSD or `Cmd+Alt+I` on macOS.
3. While pinned, use `Down`/`Right` for the next visible image path and `Up`/`Left` for the previous one.
4. Press `Esc` to close.

### Command-palette workflow

Open the palette and run **Preview Selected Image**. With no selection, the action can use the supported path under the pointer.

### Which path forms work?

Quick look is designed for ordinary local output, including:

- bare filenames from listings;
- quoted names containing spaces;
- rooted paths;
- explicit relative paths;
- Unicode filenames;
- known WSL paths that can be validated/translated for the owning Windows-hosted WSL session.

Relative paths resolve using validated working-directory metadata rather than arbitrary terminal text alone.

### Supported raster formats

Quick look supports BMP, GIF, ICO, JPEG, PNG/PNM, TIFF, and WebP raster files. Animated formats receive a static quick-look frame.

It intentionally does not open SVG, PDF, URLs, remote hosts, directories, symlinks, named pipes, or device files. Use an explicit trusted application for those cases.

## 3. Use an external application when quick look is the wrong abstraction

Choose an external application when:

- you need to edit the image;
- you need animation/full metadata/color-management behavior beyond a quick preview;
- the file is PDF/SVG or another unsupported format;
- the resource is remote and should be fetched under an explicit network/tool policy;
- the path is security-sensitive or does not pass Automexia's local regular-file checks.

Quick look is intentionally bounded; it is not a universal file opener.

## Image security boundaries users will notice

Quick look enforces these safety limits:

- no URL-scheme fetching;
- no arbitrary Windows UNC path opening;
- no symlink/device/named-pipe reads;
- a 20 MiB source-file limit;
- supported raster magic only;
- bounded dimensions/decoded pixels/allocation;
- background decode work rather than file decoding on UI/renderer/PTY threads;
- stale/replaced file checks before a decoded result is reused.

If no preview appears, check the path, format, and limits, or use an external tool.
Keep the safety checks enabled.

## Selection and copied text

Selection and copying use terminal text. Previews and backgrounds do not change
the clipboard.

For normal selection:

- drag with the mouse, or use `Shift+Arrow`;
- use `Ctrl+C` when a selection exists on Windows/Linux/BSD;
- use `Cmd+C` on macOS;
- use search when you are locating text in scrollback rather than selecting blindly.

## The pane footer is status, not a toolbar

When there is enough pane height, the footer can show information such as UTF-8, LF/CRLF, effective columns/rows, local time, pane/local-tab position, selection state, or scrollback offset.

The footer is read-only. Clicking it focuses the pane. Use shortcuts, the
scrollbar, or the palette for actions.

See [Image previews](../IMAGE-PREVIEWS.md) for format and safety limits, and
[Terminal experience](../guide/terminal-experience.md) for workspace layout.
