# Files, output, and images

Automexia helps you inspect text, files, and visual results without losing the
context of the current task. Its presentation features make output easier to
scan and images easier to preview, while the script or tool still controls the
actual bytes and behavior.

This page explains what is merely visual, what is interactive, and which image approach to use.

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

In integrated shells, Automexia owns a semantic context spacer and complete path row while the shell's line editor owns the editable command line. This separation is why resizing can reflow the prompt context without Automexia pretending to be the shell editor.

Use the prompt context for orientation, but use shell commands/environment/tooling when you need machine-readable state. Renderer labels are a view, not a scripting API.

## Icon-aware listings

Interactive `ls`/`ll` presentation can add file/folder icons and stable category colors where the shell integration supports it. Long-listing variants can present structured columns while preserving native/script behavior boundaries.

Important practical rules:

- Use the enhanced aliases/wrappers for interactive browsing.
- Use native commands/explicit script forms when exact automation semantics matter.
- In CMD, built-in `dir` is deliberately not replaced.
- A user-defined `EZA_COLORS` is not overwritten by Automexia.

If icons are missing, first check [Troubleshooting](../guide/troubleshooting.md) rather than installing unrelated shell/profile modifications.

## Semantic output

Automexia recognizes common Kubernetes, Docker, Terraform, build, test, and structured-log states and may give warnings/errors a restrained presentation treatment.

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

This is the right approach for image-aware programs such as terminal image viewers/file managers because the program controls placement, scrolling, sizing, and lifecycle as terminal content.

Examples from their respective ecosystems include `kitten icat`, `imgcat`, and `wezterm imgcat`. Automexia does not change `TERM_PROGRAM` to impersonate another terminal; if a third-party tool has not learned the `Automexia` brand, configure that tool's explicit Kitty/iTerm2/Sixel adapter rather than lying about terminal identity.

Terminal multiplexers also need to pass the chosen graphics protocol through.

## 2. Local quick look

**Available now.** Quick look is for a local raster path that is already visible in terminal output. It is an emulator-owned overlay, not bytes sent into the PTY.

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

Quick look rejects unsafe/unbounded cases rather than trying to be clever. The canonical contract includes:

- no URL-scheme fetching;
- no arbitrary Windows UNC path opening;
- no symlink/device/named-pipe reads;
- a 20 MiB source-file limit;
- supported raster magic only;
- bounded dimensions/decoded pixels/allocation;
- background decode work rather than file decoding on UI/renderer/PTY threads;
- stale/replaced file checks before a decoded result is reused.

If a preview does not appear, the correct response is to verify the path/format/limits or use an explicit external tool—not to disable the safety boundary.

## Selection and copied text

Automexia's visual enhancements do not become hidden clipboard transformations. Selection belongs to terminal cells/text; quick-look overlays and semantic backgrounds are presentation layers.

For normal selection:

- drag with the mouse, or use `Shift+Arrow`;
- use `Ctrl+C` when a selection exists on Windows/Linux/BSD;
- use `Cmd+C` on macOS;
- use search when you are locating text in scrollback rather than selecting blindly.

## The pane footer is status, not a toolbar

When there is enough pane height, the footer can show information such as UTF-8, LF/CRLF, effective columns/rows, local time, pane/local-tab position, selection state, or scrollback offset.

It is intentionally read-only. Clicking it focuses the pane; it is not a collection of invisible action buttons. Use keyboard bindings, scrollbar, or the command palette for actions.

For the detailed image limits, layout behavior, icon taxonomy, and semantic rendering contract, see [Terminal experience](../guide/terminal-experience.md).
