# Image previews

Automexia has two complementary image paths. They solve different problems and
must not be confused.

## Inline terminal graphics

A program running in the PTY can explicitly send image escape sequences. The
inherited Rio engine parses the sequence, stores bounded image state, and the
Automexia renderer places the resulting texture inside the owning pane. Images
scroll, clip, clear, and switch screen buffers with terminal content.

Supported protocol families are:

- Kitty Graphics, including direct placements and Unicode placeholders;
- iTerm2 OSC 1337 inline images, including pixel/cell/percentage sizing and the
  `doNotMoveCursor=1` extension;
- Sixel.

This is the normal path for image-aware programs. Kitty's official protocol
uses `kitten icat image.png` as its basic example; iTerm2 uses `imgcat`; WezTerm
provides `wezterm imgcat image.png`; and file managers such as Yazi render the
currently selected file through an appropriate terminal graphics protocol.
Automexia preserves its own `TERM_PROGRAM=Automexia` identity and does not
pretend to be Kitty, Rio, iTerm2, or WezTerm. A client can query the advertised
`sixel`, `iterm2`, or `kitty` terminal capabilities or explicitly emit one of
the supported protocols.

Protocol support does not mean that every third-party application recognizes a
new terminal brand automatically. If an application's detection table has not
yet learned Automexia, use its explicit Kitty/iTerm2/Sixel adapter setting
rather than changing `TERM_PROGRAM` or `TERM` to impersonate another terminal.
Multiplexers also need to pass the selected graphics protocol through.

## Local quick look

Automexia also previews a local raster path already visible in terminal output.
This is an emulator-owned overlay; it does not write escape sequences into the
PTY or alter scrollback.

- Windows, Linux, and BSD: hold `Alt` over a supported image path for 350 ms.
- macOS: hold `Cmd` over a supported image path for 350 ms.
- Keyboard: select a path and press `Ctrl`+`Alt`+`I` on Windows/Linux/BSD or
  `Cmd`+`Alt`+`I` on macOS.
- Command palette: run **Preview Selected Image**. With no selection, the
  command uses the supported path under the pointer.

Bare filenames from ordinary `ls`, quoted names containing spaces, rooted
paths, and explicit relative paths are supported. Relative paths resolve
against validated OSC 7 current-directory metadata, then the immutable launch
directory while the first prompt is still starting. On Windows, a known WSL
session may translate `/mnt/<drive>/...` to the local drive or another absolute
Linux path through `\\wsl.localhost\\<validated-distro>\\...`.

The card keeps the source aspect ratio, never enlarges a small image, limits a
large GPU upload to 1280x960, flips at pane edges, and disappears instead of
overlapping the terminal when the pane is physically unusable. It is dismissed
by typing, clicking, scrolling, selecting, leaving the window, releasing the
modifier, switching routes, or replacing the target. Resizing recomputes its
position from the current pane rectangle.

Quick look supports BMP, GIF, ICO, JPEG, PNG/PNM, TIFF, and WebP raster files.
It intentionally does not open SVG, PDF, URLs, remote hosts, directories,
symlinks, named pipes, or device files. Use an explicit trusted application for
those formats. Animated formats receive a static quick-look frame; protocol
clients own richer playback behavior.

## Security and performance contract

Terminal text is attacker-controlled, even when it comes from a local command.
Consequently quick look:

- performs no filesystem access during ordinary pointer movement;
- requires an explicit modifier dwell or keyboard/palette action;
- rejects URL schemes, arbitrary Windows UNC paths, control characters,
  symlinks, and non-regular files;
- limits the source file to 20 MiB, dimensions to 4096x4096, decoded pixels to
  16,777,216, and decoder allocation to 96 MiB;
- decodes and downsizes on one bounded worker with a bounded result queue;
- attaches every request/result to the exact route and generation, discarding
  stale work after pointer, directory, tab, pane, or session changes;
- uploads only the accepted thumbnail and removes its CPU/GPU cache entry on
  dismissal.

The iTerm2 protocol decoder separately caps decoded input at 64 MiB, validates
an optional declared `size`, and applies the same 4096x4096 and 96 MiB decoder
limits before constructing terminal graphics. Kitty/Sixel retain their
existing protocol-specific payload, dimension, and shared graphics-quota
checks.

## Verification

Focused checks are:

```text
cargo test -p automexia-terminal image_preview --locked -- --test-threads=1
cargo test -p automexia-terminal bindings --locked
cargo test -p automexia-terminal command_palette --locked
cargo test -p rio-vt --features graphics bounded_decoder --locked
cargo xtask verify architecture
```

The full `cargo ready` gate remains required before merge. Native visual review
must cover a small and large PNG, a filename containing spaces/Unicode, a WSL
path, pointer-edge flipping, extreme pane sizes, split isolation, rapid resize,
keyboard dismissal, corrupt/oversized input, and a protocol client. A visual
review does not replace the deterministic route, geometry, memory, and decoder
tests.

## Primary references

- Kitty Graphics Protocol: <https://sw.kovidgoyal.net/kitty/graphics-protocol/>
- iTerm2 inline image protocol: <https://iterm2.com/documentation-images.html>
- WezTerm iTerm image support: <https://wezterm.org/imgcat.html>
- Yazi image preview adapters: <https://yazi-rs.github.io/docs/image-preview/>>
